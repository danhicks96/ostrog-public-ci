//! Offline verification for Ostrog release manifests.
//!
//! The manifest verifier and launcher boundaries contain no credential handling. A launcher can
//! fetch a manifest and stream an archive to a temporary file separately, then verify both before
//! staging an update. The signed payload is a fixed field order rather than a serialized map, so
//! equivalent JSON formatting cannot change what is authenticated.

use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    fmt,
    fs::File,
    io::{Cursor, Read},
    path::Path,
    str::FromStr,
};

mod archive;
mod policy;
mod save_guard;
mod staging;
mod transport;

pub use archive::{unpack_archive, unpack_archive_file, ArchiveError, MAX_UNPACKED_BYTES};
pub use policy::{decide, CheckConfig, CheckDecision, FallbackReason, FetchResult, PolicyError};
pub use save_guard::{
    godot_user_dir, godot_user_dir_for, save_write_in_progress, SaveLockError, SaveWriteLock,
    SAVE_LOCK_NAME,
};
pub use staging::{
    clear_finished_update, CommittedUpdate, InstallLayout, LayoutError, StagedUpdate, StagingError,
};
pub use transport::{
    fetch_archive, fetch_archive_to_file, fetch_manifest, stream_reader_to_file, DownloadProgress,
    DownloadedArchive, FetchError, MAX_ARCHIVE_BYTES, MAX_MANIFEST_BYTES,
};

pub const MANIFEST_SCHEMA: u8 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Channel {
    Stable,
    Beta,
    Nightly,
}

impl Channel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Beta => "beta",
            Self::Nightly => "nightly",
        }
    }
}

impl FromStr for Channel {
    type Err = ManifestError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "stable" => Ok(Self::Stable),
            "beta" => Ok(Self::Beta),
            "nightly" => Ok(Self::Nightly),
            _ => Err(ManifestError::InvalidChannel(value.to_owned())),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Platform {
    Windows,
    Linux,
    Macos,
}

impl Platform {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Windows => "windows",
            Self::Linux => "linux",
            Self::Macos => "macos",
        }
    }
}

impl FromStr for Platform {
    type Err = ManifestError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "windows" => Ok(Self::Windows),
            "linux" => Ok(Self::Linux),
            "macos" => Ok(Self::Macos),
            _ => Err(ManifestError::InvalidPlatform(value.to_owned())),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Architecture {
    X86_64,
    Aarch64,
}

impl Architecture {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::X86_64 => "x86_64",
            Self::Aarch64 => "aarch64",
        }
    }
}

impl FromStr for Architecture {
    type Err = ManifestError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "x86_64" => Ok(Self::X86_64),
            "aarch64" => Ok(Self::Aarch64),
            _ => Err(ManifestError::InvalidArchitecture(value.to_owned())),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseManifest {
    pub version: String,
    pub channel: Channel,
    pub platform: Platform,
    pub architecture: Architecture,
    pub archive_url: String,
    archive_sha256: [u8; 32],
    signature: Signature,
}

impl ReleaseManifest {
    /// Parses the strict JSON manifest and validates all fields that are independent of a key.
    pub fn parse(json: &str) -> Result<Self, ManifestError> {
        let raw: RawManifest = serde_json::from_str(json)
            .map_err(|error| ManifestError::InvalidJson(error.to_string()))?;

        if raw.schema != MANIFEST_SCHEMA {
            return Err(ManifestError::UnsupportedSchema(raw.schema));
        }
        validate_version(&raw.version)?;
        validate_archive_url(&raw.archive_url)?;

        Ok(Self {
            version: raw.version,
            channel: raw.channel.parse()?,
            platform: raw.platform.parse()?,
            architecture: raw.architecture.parse()?,
            archive_url: raw.archive_url,
            archive_sha256: decode_hex::<32>(&raw.archive_sha256, "archive_sha256")?,
            signature: decode_signature(&raw.signature)?,
        })
    }

    /// Returns the exact bytes authenticated by the Ed25519 signature.
    pub fn signing_bytes(&self) -> Vec<u8> {
        manifest_signing_payload(
            &self.version,
            self.channel,
            self.platform,
            self.architecture,
            &self.archive_url,
            &hex::encode(self.archive_sha256),
        )
    }

    /// Verifies the manifest signature without downloading the archive.
    ///
    /// A launcher must perform this preflight before using the signed URL or deciding whether to
    /// download anything. Archive bytes remain untrusted until [`Self::verify_windows_x64`] or
    /// [`Self::verify_windows_x64_file`].
    pub fn verify_signature(&self, public_key: &[u8; 32]) -> Result<(), VerificationError> {
        let key = VerifyingKey::from_bytes(public_key)
            .map_err(|_| VerificationError::InvalidPublicKey)?;
        key.verify(&self.signing_bytes(), &self.signature)
            .map_err(|_| VerificationError::SignatureInvalid)
    }

    /// Verifies the signed manifest's target before the launcher starts an archive fetch.
    pub fn verify_windows_x64_target(
        &self,
        public_key: &[u8; 32],
        expected_channel: Channel,
    ) -> Result<(), VerificationError> {
        if self.channel != expected_channel {
            return Err(VerificationError::ChannelMismatch {
                expected: expected_channel,
                actual: self.channel,
            });
        }
        if self.platform != Platform::Windows || self.architecture != Architecture::X86_64 {
            return Err(VerificationError::UnsupportedTarget {
                platform: self.platform,
                architecture: self.architecture,
            });
        }
        self.verify_signature(public_key)
    }

    /// Verifies a Windows x64 release against the expected channel/version and archive bytes.
    ///
    /// The caller supplies the trusted public key. No GitHub token, URL, or download behavior is
    /// part of this crate.
    pub fn verify_windows_x64(
        &self,
        public_key: &[u8; 32],
        archive: &[u8],
        expected_version: &str,
        expected_channel: Channel,
    ) -> Result<(), VerificationError> {
        let mut reader = Cursor::new(archive);
        self.verify_windows_x64_reader(public_key, &mut reader, expected_version, expected_channel)
    }

    /// Verifies a Windows x64 release against an archive file on disk by streaming its SHA-256
    /// digest without reading the whole file into memory.
    pub fn verify_windows_x64_file(
        &self,
        public_key: &[u8; 32],
        archive_path: &Path,
        expected_version: &str,
        expected_channel: Channel,
    ) -> Result<(), VerificationError> {
        let mut file = File::open(archive_path)
            .map_err(|error| VerificationError::ArchiveIo(error.to_string()))?;
        self.verify_windows_x64_reader(public_key, &mut file, expected_version, expected_channel)
    }

    fn verify_windows_x64_reader<R: Read>(
        &self,
        public_key: &[u8; 32],
        reader: &mut R,
        expected_version: &str,
        expected_channel: Channel,
    ) -> Result<(), VerificationError> {
        validate_version(expected_version)
            .map_err(|_| VerificationError::InvalidExpectedVersion(expected_version.to_owned()))?;

        if self.version != expected_version {
            return Err(VerificationError::VersionMismatch {
                expected: expected_version.to_owned(),
                actual: self.version.clone(),
            });
        }
        self.verify_windows_x64_target(public_key, expected_channel)?;

        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 64 * 1024];
        loop {
            let read = reader
                .read(&mut buffer)
                .map_err(|error| VerificationError::ArchiveIo(error.to_string()))?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
        }
        let actual_hash = hasher.finalize();
        if actual_hash.as_slice() != self.archive_sha256 {
            return Err(VerificationError::ArchiveHashMismatch {
                expected: hex::encode(self.archive_sha256),
                actual: hex::encode(actual_hash),
            });
        }

        Ok(())
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum ManifestError {
    InvalidJson(String),
    UnsupportedSchema(u8),
    InvalidVersion(String),
    InvalidChannel(String),
    InvalidPlatform(String),
    InvalidArchitecture(String),
    InvalidArchiveUrl(String),
    InvalidHashEncoding(String),
    InvalidSignatureEncoding,
}

#[derive(Debug, Eq, PartialEq)]
pub enum VerificationError {
    InvalidExpectedVersion(String),
    VersionMismatch {
        expected: String,
        actual: String,
    },
    ChannelMismatch {
        expected: Channel,
        actual: Channel,
    },
    UnsupportedTarget {
        platform: Platform,
        architecture: Architecture,
    },
    ArchiveHashMismatch {
        expected: String,
        actual: String,
    },
    ArchiveIo(String),
    InvalidPublicKey,
    SignatureInvalid,
}

pub fn validate_version(version: &str) -> Result<(), ManifestError> {
    let mut parts = version.split('.');
    let valid = (0..3).all(|_| {
        parts.next().is_some_and(|part| {
            !part.is_empty()
                && (part == "0" || !part.starts_with('0'))
                && part.bytes().all(|byte| byte.is_ascii_digit())
        })
    }) && parts.next().is_none();

    if valid {
        Ok(())
    } else {
        Err(ManifestError::InvalidVersion(version.to_owned()))
    }
}

fn validate_archive_url(url: &str) -> Result<(), ManifestError> {
    if is_valid_https_url(url) {
        Ok(())
    } else {
        Err(ManifestError::InvalidArchiveUrl(url.to_owned()))
    }
}

pub(crate) fn is_valid_https_url(url: &str) -> bool {
    let Some(authority_and_path) = url.strip_prefix("https://") else {
        return false;
    };
    let authority = authority_and_path
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default();
    !authority.is_empty()
        && !authority.contains('@')
        && !authority.contains('\\')
        && !url.bytes().any(|byte| byte.is_ascii_control())
}

pub(crate) fn version_is_newer(candidate: &str, current: &str) -> bool {
    candidate
        .split('.')
        .zip(current.split('.'))
        .find_map(|(candidate_part, current_part)| {
            (candidate_part.len() != current_part.len())
                .then(|| candidate_part.len().cmp(&current_part.len()))
                .or_else(|| {
                    (candidate_part != current_part).then(|| candidate_part.cmp(current_part))
                })
        })
        .is_some_and(|ordering| ordering.is_gt())
}

pub fn manifest_signing_payload(
    version: &str,
    channel: Channel,
    platform: Platform,
    architecture: Architecture,
    archive_url: &str,
    archive_sha256: &str,
) -> Vec<u8> {
    format!(
        "schema={MANIFEST_SCHEMA}\nversion={version}\nchannel={}\nplatform={}\narchitecture={}\narchive_url={archive_url}\narchive_sha256={archive_sha256}\n",
        channel.as_str(),
        platform.as_str(),
        architecture.as_str(),
    )
    .into_bytes()
}

fn decode_hex<const N: usize>(value: &str, field: &str) -> Result<[u8; N], ManifestError> {
    let bytes = hex::decode(value).map_err(|_| ManifestError::InvalidHashEncoding(field.into()))?;
    bytes
        .try_into()
        .map_err(|_| ManifestError::InvalidHashEncoding(field.into()))
}

fn decode_signature(value: &str) -> Result<Signature, ManifestError> {
    let bytes = hex::decode(value).map_err(|_| ManifestError::InvalidSignatureEncoding)?;
    let bytes: [u8; 64] = bytes
        .try_into()
        .map_err(|_| ManifestError::InvalidSignatureEncoding)?;
    Ok(Signature::from_bytes(&bytes))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawManifest {
    schema: u8,
    version: String,
    channel: String,
    platform: String,
    architecture: String,
    archive_url: String,
    archive_sha256: String,
    signature: String,
}

impl fmt::Display for ManifestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for ManifestError {}

impl fmt::Display for VerificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for VerificationError {}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use serde_json::json;

    const ARCHIVE: &[u8] = b"ostrog-windows-x64.zip";
    const ARCHIVE_URL: &str = "https://releases.example/ostrog-windows-x64.zip";

    fn manifest_json(
        signing_key: &SigningKey,
        version: &str,
        channel: &str,
        platform: &str,
        architecture: &str,
        archive: &[u8],
    ) -> String {
        let archive_hash = hex::encode(Sha256::digest(archive));
        let channel = channel.parse().unwrap();
        let platform = platform.parse().unwrap();
        let architecture = architecture.parse().unwrap();
        let payload = manifest_signing_payload(
            version,
            channel,
            platform,
            architecture,
            ARCHIVE_URL,
            &archive_hash,
        );
        let signature = signing_key.sign(&payload);
        json!({
            "schema": MANIFEST_SCHEMA,
            "version": version,
            "channel": channel.as_str(),
            "platform": platform.as_str(),
            "architecture": architecture.as_str(),
            "archive_url": ARCHIVE_URL,
            "archive_sha256": archive_hash,
            "signature": hex::encode(signature.to_bytes()),
        })
        .to_string()
    }

    fn key() -> SigningKey {
        SigningKey::from_bytes(&[7; 32])
    }

    #[test]
    fn valid_windows_manifest_verifies() {
        let signing_key = key();
        let manifest = ReleaseManifest::parse(&manifest_json(
            &signing_key,
            "1.2.3",
            "stable",
            "windows",
            "x86_64",
            ARCHIVE,
        ))
        .unwrap();

        assert_eq!(manifest.version, "1.2.3");
        assert_eq!(manifest.archive_url, ARCHIVE_URL);
        manifest
            .verify_windows_x64(
                &signing_key.verifying_key().to_bytes(),
                ARCHIVE,
                "1.2.3",
                Channel::Stable,
            )
            .unwrap();
        manifest
            .verify_windows_x64_target(&signing_key.verifying_key().to_bytes(), Channel::Stable)
            .unwrap();
    }

    #[test]
    fn signature_preflight_rejects_tampering_before_archive_download() {
        let signing_key = key();
        let mut value: serde_json::Value = serde_json::from_str(&manifest_json(
            &signing_key,
            "1.2.3",
            "stable",
            "windows",
            "x86_64",
            ARCHIVE,
        ))
        .unwrap();
        value["archive_url"] = json!("https://cdn.example/other.zip");
        let manifest = ReleaseManifest::parse(&value.to_string()).unwrap();
        assert_eq!(
            manifest.verify_windows_x64_target(
                &signing_key.verifying_key().to_bytes(),
                Channel::Stable
            ),
            Err(VerificationError::SignatureInvalid)
        );
    }

    #[test]
    fn malformed_and_tampered_signatures_are_rejected() {
        let signing_key = key();
        let mut value: serde_json::Value = serde_json::from_str(&manifest_json(
            &signing_key,
            "1.2.3",
            "stable",
            "windows",
            "x86_64",
            ARCHIVE,
        ))
        .unwrap();
        value["signature"] = json!("not-a-signature");
        assert_eq!(
            ReleaseManifest::parse(&value.to_string()),
            Err(ManifestError::InvalidSignatureEncoding)
        );

        let mut tampered: serde_json::Value = serde_json::from_str(&manifest_json(
            &signing_key,
            "1.2.3",
            "stable",
            "windows",
            "x86_64",
            ARCHIVE,
        ))
        .unwrap();
        let signature = tampered["signature"].as_str().unwrap();
        let replacement = format!("{}{}", &"00".repeat(63), &signature[126..]);
        tampered["signature"] = json!(replacement);
        let manifest = ReleaseManifest::parse(&tampered.to_string()).unwrap();
        assert_eq!(
            manifest.verify_windows_x64(
                &signing_key.verifying_key().to_bytes(),
                ARCHIVE,
                "1.2.3",
                Channel::Stable,
            ),
            Err(VerificationError::SignatureInvalid)
        );
    }

    #[test]
    fn changing_archive_url_invalidates_the_manifest_signature() {
        let signing_key = key();
        let mut value: serde_json::Value = serde_json::from_str(&manifest_json(
            &signing_key,
            "1.2.3",
            "stable",
            "windows",
            "x86_64",
            ARCHIVE,
        ))
        .unwrap();
        value["archive_url"] = json!("https://cdn.example/another.zip");
        let manifest = ReleaseManifest::parse(&value.to_string()).unwrap();

        assert_eq!(
            manifest.verify_windows_x64(
                &signing_key.verifying_key().to_bytes(),
                ARCHIVE,
                "1.2.3",
                Channel::Stable,
            ),
            Err(VerificationError::SignatureInvalid)
        );
    }

    #[test]
    fn manifest_rejects_non_https_archive_urls() {
        let signing_key = key();
        let invalid_url = manifest_json(
            &signing_key,
            "1.2.3",
            "stable",
            "windows",
            "x86_64",
            ARCHIVE,
        )
        .replace(
            ARCHIVE_URL,
            "http://releases.example/ostrog-windows-x64.zip",
        );

        assert_eq!(
            ReleaseManifest::parse(&invalid_url),
            Err(ManifestError::InvalidArchiveUrl(
                "http://releases.example/ostrog-windows-x64.zip".into()
            ))
        );
    }

    #[test]
    fn wrong_archive_hash_is_rejected() {
        let signing_key = key();
        let manifest = ReleaseManifest::parse(&manifest_json(
            &signing_key,
            "1.2.3",
            "stable",
            "windows",
            "x86_64",
            ARCHIVE,
        ))
        .unwrap();

        assert!(matches!(
            manifest.verify_windows_x64(
                &signing_key.verifying_key().to_bytes(),
                b"a different archive",
                "1.2.3",
                Channel::Stable,
            ),
            Err(VerificationError::ArchiveHashMismatch { .. })
        ));
    }

    #[test]
    fn wrong_platform_is_rejected() {
        let signing_key = key();
        let manifest = ReleaseManifest::parse(&manifest_json(
            &signing_key,
            "1.2.3",
            "stable",
            "linux",
            "x86_64",
            ARCHIVE,
        ))
        .unwrap();

        assert_eq!(
            manifest.verify_windows_x64(
                &signing_key.verifying_key().to_bytes(),
                ARCHIVE,
                "1.2.3",
                Channel::Stable,
            ),
            Err(VerificationError::UnsupportedTarget {
                platform: Platform::Linux,
                architecture: Architecture::X86_64,
            })
        );
    }

    #[test]
    fn invalid_version_and_channel_are_rejected() {
        let signing_key = key();
        let invalid_version = manifest_json(
            &signing_key,
            "01.2.3",
            "stable",
            "windows",
            "x86_64",
            ARCHIVE,
        );
        assert_eq!(
            ReleaseManifest::parse(&invalid_version),
            Err(ManifestError::InvalidVersion("01.2.3".into()))
        );

        let invalid_channel = manifest_json(
            &signing_key,
            "1.2.3",
            "stable",
            "windows",
            "x86_64",
            ARCHIVE,
        )
        .replace("\"channel\":\"stable\"", "\"channel\":\"preview\"");
        assert_eq!(
            ReleaseManifest::parse(&invalid_channel),
            Err(ManifestError::InvalidChannel("preview".into()))
        );
    }
}
