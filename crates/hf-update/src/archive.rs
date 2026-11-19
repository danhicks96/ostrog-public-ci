//! Safe extraction of a verified release archive into a disposable directory.

use std::{
    collections::BTreeSet,
    fs::{self, File},
    io::{self, Cursor, Read, Seek, Write},
    path::{Component, Path, PathBuf},
};

use zip::ZipArchive;

/// The maximum uncompressed tree accepted from one release archive.
pub const MAX_UNPACKED_BYTES: u64 = 512 * 1024 * 1024;

#[derive(Debug, Eq, PartialEq)]
pub enum ArchiveError {
    InvalidDestination(PathBuf),
    EmptyArchive,
    InvalidEntryName(String),
    SymlinkNotAllowed(String),
    UnpackedTooLarge { maximum: u64 },
    Zip(String),
    Io(String),
}

impl std::fmt::Display for ArchiveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for ArchiveError {}

/// Extracts a release archive into a fresh disposable directory.
///
/// Paths are required to stay below `destination`, symlinks are rejected, and both each file and
/// the complete uncompressed tree are bounded. The caller must remove `destination` if this
/// returns an error; it is never treated as an installable application by this function.
pub fn unpack_archive(archive: &[u8], destination: &Path) -> Result<PathBuf, ArchiveError> {
    unpack_reader(Cursor::new(archive), destination)
}

/// Extracts a release archive file on disk into a fresh disposable directory without reading the
/// whole compressed archive into memory.
pub fn unpack_archive_file(
    archive_path: &Path,
    destination: &Path,
) -> Result<PathBuf, ArchiveError> {
    let file = File::open(archive_path).map_err(io_error)?;
    unpack_reader(file, destination)
}

fn unpack_reader<R: Read + Seek>(reader: R, destination: &Path) -> Result<PathBuf, ArchiveError> {
    if destination.exists() {
        return Err(ArchiveError::InvalidDestination(destination.to_owned()));
    }
    fs::create_dir_all(destination).map_err(io_error)?;

    let mut zip = ZipArchive::new(reader).map_err(zip_error)?;
    let mut total_unpacked = 0u64;
    let mut roots = BTreeSet::new();

    for index in 0..zip.len() {
        let mut entry = zip.by_index(index).map_err(zip_error)?;
        let name = entry.name().map_err(zip_error)?.into_owned();
        if entry.is_symlink() {
            return Err(ArchiveError::SymlinkNotAllowed(name));
        }

        let relative = safe_entry_path(&name)?;
        let Some(root) = relative
            .components()
            .next()
            .and_then(|component| match component {
                Component::Normal(root) => Some(root.to_owned()),
                _ => None,
            })
        else {
            return Err(ArchiveError::InvalidEntryName(name));
        };
        roots.insert(root);
        let output = destination.join(&relative);

        if entry.is_dir() {
            fs::create_dir_all(&output).map_err(io_error)?;
            continue;
        }

        if output.parent().is_none() {
            return Err(ArchiveError::InvalidEntryName(name));
        }
        fs::create_dir_all(output.parent().expect("checked above")).map_err(io_error)?;
        let mut file = File::create(&output).map_err(io_error)?;
        let written = copy_bounded(&mut entry, &mut file, &mut total_unpacked)?;
        if written == 0 && entry.size() != 0 {
            return Err(ArchiveError::Zip("archive entry ended early".into()));
        }
        file.flush().map_err(io_error)?;
    }

    if roots.is_empty() {
        return Err(ArchiveError::EmptyArchive);
    }
    let package_root = if roots.len() == 1 {
        destination.join(roots.into_iter().next().expect("one root"))
    } else {
        destination.to_owned()
    };
    Ok(package_root)
}

fn safe_entry_path(name: &str) -> Result<PathBuf, ArchiveError> {
    if name.is_empty() || name.contains('\\') || name.starts_with('/') || name.contains(':') {
        return Err(ArchiveError::InvalidEntryName(name.to_owned()));
    }
    let path = Path::new(name);
    for component in path.components() {
        if matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        ) {
            return Err(ArchiveError::InvalidEntryName(name.to_owned()));
        }
    }
    Ok(path.to_owned())
}

fn copy_bounded(
    reader: &mut impl Read,
    writer: &mut impl Write,
    total: &mut u64,
) -> Result<u64, ArchiveError> {
    let mut buffer = [0u8; 64 * 1024];
    let mut written = 0u64;
    loop {
        let read = reader.read(&mut buffer).map_err(io_error)?;
        if read == 0 {
            return Ok(written);
        }
        let read = u64::try_from(read).expect("buffer length fits u64");
        if read > MAX_UNPACKED_BYTES - (*total).min(MAX_UNPACKED_BYTES) {
            return Err(ArchiveError::UnpackedTooLarge {
                maximum: MAX_UNPACKED_BYTES,
            });
        }
        writer
            .write_all(&buffer[..read as usize])
            .map_err(io_error)?;
        *total += read;
        written += read;
    }
}

fn io_error(error: io::Error) -> ArchiveError {
    ArchiveError::Io(error.to_string())
}

fn zip_error(error: zip::result::ZipError) -> ArchiveError {
    ArchiveError::Zip(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use zip::write::{SimpleFileOptions, ZipWriter};

    fn archive(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut bytes = Cursor::new(Vec::new());
        {
            let mut writer = ZipWriter::new(&mut bytes);
            for (name, contents) in entries {
                writer
                    .start_file(*name, SimpleFileOptions::default())
                    .unwrap();
                writer.write_all(contents).unwrap();
            }
            writer.finish().unwrap();
        }
        bytes.into_inner()
    }

    #[test]
    fn packaged_archive_returns_its_single_application_root() {
        let tree = std::env::temp_dir().join(format!(
            "ostrog-unpack-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let bytes = archive(&[("ostrog-windows-x64/ostrog.exe", b"game")]);
        let root = unpack_archive(&bytes, &tree).unwrap();
        assert_eq!(root, tree.join("ostrog-windows-x64"));
        assert_eq!(fs::read(root.join("ostrog.exe")).unwrap(), b"game");
        fs::remove_dir_all(tree).unwrap();
    }

    #[test]
    fn traversal_and_backslash_paths_are_rejected() {
        let tree = std::env::temp_dir().join(format!("ostrog-unpack-bad-{}", std::process::id()));
        for name in ["../escape.txt", "ostrog\\escape.txt"] {
            let error = unpack_archive(&archive(&[(name, b"bad")]), &tree).unwrap_err();
            assert_eq!(error, ArchiveError::InvalidEntryName(name.to_owned()));
            let _ = fs::remove_dir_all(&tree);
        }
    }

    #[test]
    fn an_existing_destination_is_not_overwritten() {
        let tree =
            std::env::temp_dir().join(format!("ostrog-unpack-existing-{}", std::process::id()));
        fs::create_dir_all(&tree).unwrap();
        assert_eq!(
            unpack_archive(&archive(&[("game.exe", b"new")]), &tree),
            Err(ArchiveError::InvalidDestination(tree.clone()))
        );
        fs::remove_dir_all(tree).unwrap();
    }

    #[test]
    fn an_empty_archive_is_not_an_installable_tree() {
        let tree = std::env::temp_dir().join(format!("ostrog-unpack-empty-{}", std::process::id()));
        let mut bytes = Cursor::new(Vec::new());
        ZipWriter::new(&mut bytes).finish().unwrap();
        assert_eq!(
            unpack_archive(&bytes.into_inner(), &tree),
            Err(ArchiveError::EmptyArchive)
        );
        fs::remove_dir_all(tree).unwrap();
    }
}
