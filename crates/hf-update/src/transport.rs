//! Anonymous, bounded HTTPS retrieval for the signed release manifest and streamed archives.
//!
//! This module retrieves public release metadata and streams archive bytes to a temporary file.
//! The caller still supplies the trusted public key and passes the downloaded archive to
//! [`crate::ReleaseManifest::verify_windows_x64_file`] before staging anything. There is no GitHub
//! API client, token, redirect following, whole-archive memory buffer, or executable launch here.

use crate::{is_valid_https_url, CheckConfig, ManifestError, PolicyError, ReleaseManifest};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub const MAX_MANIFEST_BYTES: u64 = 64 * 1024;
/// Hard upper bound for one downloaded Windows release archive.
///
/// The package is still untrusted until its signed manifest and SHA-256 have been checked. This
/// limit only protects the launcher from an unexpectedly large response or a server-side mistake.
pub const MAX_ARCHIVE_BYTES: u64 = 512 * 1024 * 1024;

const STREAM_CHUNK_BYTES: usize = 64 * 1024;

static DOWNLOAD_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Byte progress reported as a release archive streams to disk.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DownloadProgress {
    pub bytes_downloaded: u64,
    pub total_bytes: Option<u64>,
}

/// A downloaded release archive stored in a disposable temporary file on disk.
///
/// Dropping this handle removes the temporary file unless [`Self::into_path`] is called.
#[derive(Debug)]
pub struct DownloadedArchive {
    path: PathBuf,
    bytes: u64,
}

impl DownloadedArchive {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn bytes(&self) -> u64 {
        self.bytes
    }

    pub fn into_path(mut self) -> PathBuf {
        std::mem::take(&mut self.path)
    }

    /// Creates a disposable temporary archive file from `bytes` (for deterministic tests).
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, FetchError> {
        let path = next_temp_archive_path();
        let mut reader = std::io::Cursor::new(bytes);
        let written = stream_reader_to_file(
            &mut reader,
            &path,
            Some(bytes.len() as u64),
            MAX_ARCHIVE_BYTES,
            |_| {},
        )?;
        Ok(Self {
            path,
            bytes: written,
        })
    }
}

impl Drop for DownloadedArchive {
    fn drop(&mut self) {
        if !self.path.as_os_str().is_empty() {
            let _ = fs::remove_file(&self.path);
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum FetchError {
    InvalidUrl(String),
    InvalidConfig(PolicyError),
    Transport(String),
    HttpStatus(u16),
    ResponseTooLarge { maximum: u64 },
    TruncatedResponse { expected: u64, actual: u64 },
    InvalidContentLength(String),
    Io(String),
    InvalidUtf8,
    InvalidManifest(ManifestError),
}

impl std::fmt::Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for FetchError {}

/// Fetches one public manifest over HTTPS with no credentials and a bounded response.
pub fn fetch_manifest(url: &str, config: CheckConfig) -> Result<ReleaseManifest, FetchError> {
    let bytes = fetch_bytes(url, config, MAX_MANIFEST_BYTES)?;
    let json = String::from_utf8(bytes).map_err(|_| FetchError::InvalidUtf8)?;
    ReleaseManifest::parse(&json).map_err(FetchError::InvalidManifest)
}

/// Streams one public release archive over HTTPS with no credentials and a bounded response into
/// a disposable temporary file, invoking `on_progress` as chunks are written.
///
/// The downloaded file is not trusted package content. Callers must verify it against the signed
/// [`ReleaseManifest`] before extracting or staging it.
pub fn fetch_archive<F>(
    url: &str,
    config: CheckConfig,
    on_progress: F,
) -> Result<DownloadedArchive, FetchError>
where
    F: FnMut(DownloadProgress),
{
    let path = next_temp_archive_path();
    let bytes = fetch_archive_to_file(url, config, &path, on_progress)?;
    Ok(DownloadedArchive { path, bytes })
}

/// Streams one public release archive over HTTPS into `destination`, enforcing
/// [`MAX_ARCHIVE_BYTES`], checking `Content-Length` truncation, and reporting progress.
/// Removes any partial file at `destination` if an error occurs.
pub fn fetch_archive_to_file<F>(
    url: &str,
    config: CheckConfig,
    destination: &Path,
    on_progress: F,
) -> Result<u64, FetchError>
where
    F: FnMut(DownloadProgress),
{
    let response = get_response(url, config)?;
    let expected_total = parse_content_length(response.header("Content-Length"))?;
    let mut reader = response.into_reader();
    stream_reader_to_file(
        &mut reader,
        destination,
        expected_total,
        MAX_ARCHIVE_BYTES,
        on_progress,
    )
}

/// Streams `reader` to `destination` in fixed 64 KiB chunks without allocating a whole-archive
/// `Vec`, enforcing `maximum` and `expected_total` (when known) and invoking `on_progress` after
/// each chunk. Removes `destination` on error.
pub fn stream_reader_to_file<R, F>(
    reader: &mut R,
    destination: &Path,
    expected_total: Option<u64>,
    maximum: u64,
    mut on_progress: F,
) -> Result<u64, FetchError>
where
    R: Read + ?Sized,
    F: FnMut(DownloadProgress),
{
    if let Some(total) = expected_total {
        if total > maximum {
            return Err(FetchError::ResponseTooLarge { maximum });
        }
    }
    if let Some(parent) = destination.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|error| FetchError::Io(error.to_string()))?;
        }
    }
    let mut file = File::create(destination).map_err(|error| FetchError::Io(error.to_string()))?;
    let mut buffer = [0u8; STREAM_CHUNK_BYTES];
    let mut downloaded = 0u64;

    loop {
        let read = match reader.read(&mut buffer) {
            Ok(read) => read,
            Err(error) => {
                drop(file);
                let _ = fs::remove_file(destination);
                return Err(FetchError::Transport(error.to_string()));
            }
        };
        if read == 0 {
            break;
        }
        let chunk = u64::try_from(read).expect("chunk buffer fits in u64");
        let next_total = downloaded.saturating_add(chunk);
        if next_total > maximum {
            drop(file);
            let _ = fs::remove_file(destination);
            return Err(FetchError::ResponseTooLarge { maximum });
        }
        if let Some(expected) = expected_total {
            if next_total > expected {
                drop(file);
                let _ = fs::remove_file(destination);
                return Err(FetchError::TruncatedResponse {
                    expected,
                    actual: next_total,
                });
            }
        }
        if let Err(error) = file.write_all(&buffer[..read]) {
            drop(file);
            let _ = fs::remove_file(destination);
            return Err(FetchError::Io(error.to_string()));
        }
        downloaded = next_total;
        on_progress(DownloadProgress {
            bytes_downloaded: downloaded,
            total_bytes: expected_total,
        });
    }

    if let Some(expected) = expected_total {
        if downloaded < expected {
            drop(file);
            let _ = fs::remove_file(destination);
            return Err(FetchError::TruncatedResponse {
                expected,
                actual: downloaded,
            });
        }
    }

    if let Err(error) = file.flush() {
        drop(file);
        let _ = fs::remove_file(destination);
        return Err(FetchError::Io(error.to_string()));
    }

    if downloaded == 0 {
        on_progress(DownloadProgress {
            bytes_downloaded: 0,
            total_bytes: expected_total,
        });
    }

    Ok(downloaded)
}

fn next_temp_archive_path() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let seq = DOWNLOAD_COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "ostrog-archive-{}-{nonce}-{seq}.zip.part",
        std::process::id()
    ))
}

fn parse_content_length(header: Option<&str>) -> Result<Option<u64>, FetchError> {
    let Some(raw) = header else {
        return Ok(None);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(FetchError::InvalidContentLength(raw.to_owned()));
    }
    let parsed = trimmed
        .parse::<u64>()
        .map_err(|_| FetchError::InvalidContentLength(raw.to_owned()))?;
    Ok(Some(parsed))
}

fn get_response(url: &str, config: CheckConfig) -> Result<ureq::Response, FetchError> {
    validate_url(url)?;
    config.validate().map_err(FetchError::InvalidConfig)?;

    let timeout = Duration::from_secs(u64::from(config.timeout_seconds));
    let agent = ureq::AgentBuilder::new()
        .timeout(timeout)
        .redirects(0)
        .build();
    let response = match agent.get(url).call() {
        Ok(response) => response,
        Err(ureq::Error::Status(status, _)) => return Err(FetchError::HttpStatus(status)),
        Err(ureq::Error::Transport(error)) => return Err(FetchError::Transport(error.to_string())),
    };
    if response.status() != 200 {
        return Err(FetchError::HttpStatus(response.status()));
    }
    Ok(response)
}

fn fetch_bytes(url: &str, config: CheckConfig, maximum: u64) -> Result<Vec<u8>, FetchError> {
    let response = get_response(url, config)?;
    let mut reader = response.into_reader();
    read_bounded(&mut reader, maximum)
}

fn validate_url(url: &str) -> Result<(), FetchError> {
    if !is_valid_https_url(url) {
        return Err(FetchError::InvalidUrl(url.to_owned()));
    }
    Ok(())
}

fn read_bounded(reader: &mut impl Read, maximum: u64) -> Result<Vec<u8>, FetchError> {
    let mut bytes = Vec::new();
    reader
        .take(maximum + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| FetchError::Transport(error.to_string()))?;
    if bytes.len() as u64 > maximum {
        return Err(FetchError::ResponseTooLarge { maximum });
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{self, Cursor};

    fn config() -> CheckConfig {
        CheckConfig {
            enabled: true,
            timeout_seconds: 10,
            deferred_until: None,
        }
    }

    struct ChunkedReader {
        chunks: Vec<Result<Vec<u8>, io::ErrorKind>>,
        index: usize,
    }

    impl ChunkedReader {
        fn from_slices(slices: &[&[u8]]) -> Self {
            Self {
                chunks: slices.iter().map(|s| Ok(s.to_vec())).collect(),
                index: 0,
            }
        }
    }

    impl Read for ChunkedReader {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if self.index >= self.chunks.len() {
                return Ok(0);
            }
            match &self.chunks[self.index] {
                Ok(chunk) => {
                    let n = chunk.len().min(buf.len());
                    buf[..n].copy_from_slice(&chunk[..n]);
                    self.index += 1;
                    Ok(n)
                }
                Err(kind) => {
                    self.index += 1;
                    Err(io::Error::from(*kind))
                }
            }
        }
    }

    #[test]
    fn only_anonymous_https_urls_are_accepted() {
        assert!(matches!(
            fetch_manifest("http://releases.example/manifest.json", config()),
            Err(FetchError::InvalidUrl(_))
        ));
        assert!(matches!(
            fetch_manifest(
                "https://user:secret@releases.example/manifest.json",
                config()
            ),
            Err(FetchError::InvalidUrl(_))
        ));
        assert!(matches!(
            fetch_manifest("https://releases.example/manifest.json\n", config()),
            Err(FetchError::InvalidUrl(_))
        ));
    }

    #[test]
    fn archive_fetch_uses_the_same_anonymous_https_boundary() {
        assert!(matches!(
            fetch_archive("http://releases.example/ostrog.zip", config(), |_| {}),
            Err(FetchError::InvalidUrl(_))
        ));
        assert!(matches!(
            fetch_archive(
                "https://user:secret@releases.example/ostrog.zip",
                config(),
                |_| {}
            ),
            Err(FetchError::InvalidUrl(_))
        ));
        assert!(matches!(
            fetch_archive("https://releases.example/ostrog.zip\n", config(), |_| {}),
            Err(FetchError::InvalidUrl(_))
        ));
    }

    #[test]
    fn response_size_is_bounded_before_manifest_parsing() {
        let mut reader = Cursor::new(vec![b'x'; (MAX_MANIFEST_BYTES + 1) as usize]);
        assert_eq!(
            read_bounded(&mut reader, MAX_MANIFEST_BYTES),
            Err(FetchError::ResponseTooLarge {
                maximum: MAX_MANIFEST_BYTES
            })
        );
    }

    #[test]
    fn streamed_archive_enforces_size_bound_and_removes_partial_file() {
        let dest = next_temp_archive_path();
        let mut reader = ChunkedReader::from_slices(&[b"12345", b"6789"]);
        let mut progress = Vec::new();
        assert_eq!(
            stream_reader_to_file(&mut reader, &dest, None, 8, |p| progress.push(p)),
            Err(FetchError::ResponseTooLarge { maximum: 8 })
        );
        assert!(
            !dest.exists(),
            "partial temp file must be removed when archive exceeds size bound"
        );
        assert_eq!(
            progress,
            vec![DownloadProgress {
                bytes_downloaded: 5,
                total_bytes: None,
            }]
        );

        // Also rejected up front when Content-Length alone exceeds the bound.
        let mut untouched = ChunkedReader::from_slices(&[b"123"]);
        assert_eq!(
            stream_reader_to_file(&mut untouched, &dest, Some(9), 8, |_| {}),
            Err(FetchError::ResponseTooLarge { maximum: 8 })
        );
        assert!(!dest.exists());
    }

    #[test]
    fn streamed_archive_detects_truncation_and_removes_partial_file() {
        let dest = next_temp_archive_path();
        let mut short_reader = ChunkedReader::from_slices(&[b"part1", b"part2"]);
        let mut progress = Vec::new();
        assert_eq!(
            stream_reader_to_file(&mut short_reader, &dest, Some(15), 64, |p| {
                progress.push(p)
            }),
            Err(FetchError::TruncatedResponse {
                expected: 15,
                actual: 10,
            })
        );
        assert!(
            !dest.exists(),
            "truncated archive temp file must be removed on early EOF"
        );
        assert_eq!(
            progress,
            vec![
                DownloadProgress {
                    bytes_downloaded: 5,
                    total_bytes: Some(15),
                },
                DownloadProgress {
                    bytes_downloaded: 10,
                    total_bytes: Some(15),
                },
            ]
        );

        // Mid-stream transport error also cleans up the partial file.
        let mut broken_reader = ChunkedReader {
            chunks: vec![Ok(b"hello".to_vec()), Err(io::ErrorKind::ConnectionReset)],
            index: 0,
        };
        assert!(matches!(
            stream_reader_to_file(&mut broken_reader, &dest, Some(10), 64, |_| {}),
            Err(FetchError::Transport(_))
        ));
        assert!(
            !dest.exists(),
            "partial archive temp file must be removed on mid-stream transport error"
        );
    }

    #[test]
    fn streamed_archive_writes_temp_file_and_reports_progress_per_chunk() {
        let dest = next_temp_archive_path();
        let mut reader = ChunkedReader::from_slices(&[b"ostrog", b"-arch", b"ive-123"]);
        let mut progress = Vec::new();
        let written =
            stream_reader_to_file(&mut reader, &dest, Some(18), 64, |p| progress.push(p)).unwrap();
        assert_eq!(written, 18);
        assert_eq!(fs::read(&dest).unwrap(), b"ostrog-archive-123");
        assert_eq!(
            progress,
            vec![
                DownloadProgress {
                    bytes_downloaded: 6,
                    total_bytes: Some(18),
                },
                DownloadProgress {
                    bytes_downloaded: 11,
                    total_bytes: Some(18),
                },
                DownloadProgress {
                    bytes_downloaded: 18,
                    total_bytes: Some(18),
                },
            ]
        );
        fs::remove_file(&dest).unwrap();
    }

    #[test]
    fn downloaded_archive_cleans_up_temp_file_on_drop() {
        let archive = DownloadedArchive::from_bytes(b"zip-bytes").unwrap();
        let path = archive.path().to_path_buf();
        assert!(path.is_file());
        assert_eq!(archive.bytes(), 9);
        drop(archive);
        assert!(!path.exists());
    }

    #[test]
    fn timeout_policy_is_validated_before_transport() {
        let invalid = CheckConfig {
            timeout_seconds: 31,
            ..config()
        };
        assert_eq!(
            fetch_manifest("https://releases.example/manifest.json", invalid),
            Err(FetchError::InvalidConfig(PolicyError::TimeoutOutOfRange {
                seconds: 31,
                maximum: 30
            }))
        );
    }
}
