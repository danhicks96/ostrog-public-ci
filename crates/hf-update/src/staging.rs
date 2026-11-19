//! Filesystem-only update staging and recovery.
//!
//! The launcher is responsible for downloading and verifying an archive. This module starts
//! after that trust decision: it copies a verified unpacked payload to a sibling staging area,
//! swaps the application directory through a same-volume rename, and leaves the user-data
//! directory untouched. It never executes package contents and has no network or credential code.

use crate::{
    archive::{unpack_archive, unpack_archive_file},
    ArchiveError, Channel, ReleaseManifest, VerificationError,
};
use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

static STAGING_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LayoutError {
    ApplicationContainsUserData,
    UserDataInsideUpdateArea,
}

impl std::fmt::Display for LayoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for LayoutError {}

#[derive(Debug)]
pub enum StagingError {
    Layout(LayoutError),
    Io(io::Error),
    PackageMissing(PathBuf),
    PackageNotDirectory(PathBuf),
    SymlinkNotAllowed(PathBuf),
    BackupAlreadyExists(PathBuf),
    NoBackup(PathBuf),
    Verification(VerificationError),
    Archive(ArchiveError),
}

impl std::fmt::Display for StagingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for StagingError {}

impl From<io::Error> for StagingError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<ArchiveError> for StagingError {
    fn from(error: ArchiveError) -> Self {
        Self::Archive(error)
    }
}

/// The application and user-data paths used by a portable install.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstallLayout {
    application_dir: PathBuf,
    user_data_dir: PathBuf,
    update_dir: PathBuf,
}

impl InstallLayout {
    /// Creates a layout whose update area is a sibling of the running application directory.
    /// User data may not be inside either the application or update area, because both areas can
    /// be renamed or cleaned during recovery.
    pub fn new(
        application_dir: impl Into<PathBuf>,
        user_data_dir: impl Into<PathBuf>,
    ) -> Result<Self, LayoutError> {
        let application_dir = application_dir.into();
        let user_data_dir = user_data_dir.into();
        let update_dir = application_dir.parent().map_or_else(
            || PathBuf::from(".ostrog-update"),
            |parent| parent.join(".ostrog-update"),
        );

        if is_same_or_descendant(&user_data_dir, &application_dir) {
            return Err(LayoutError::ApplicationContainsUserData);
        }
        if is_same_or_descendant(&user_data_dir, &update_dir) {
            return Err(LayoutError::UserDataInsideUpdateArea);
        }

        Ok(Self {
            application_dir,
            user_data_dir,
            update_dir,
        })
    }

    pub fn application_dir(&self) -> &Path {
        &self.application_dir
    }

    pub fn user_data_dir(&self) -> &Path {
        &self.user_data_dir
    }

    pub fn update_dir(&self) -> &Path {
        &self.update_dir
    }

    /// Copies a verified, unpacked package into a fresh sibling staging directory.
    pub fn stage_from_dir(
        &self,
        package_dir: impl AsRef<Path>,
    ) -> Result<StagedUpdate, StagingError> {
        let package_dir = package_dir.as_ref();
        let metadata = fs::symlink_metadata(package_dir).map_err(|error| {
            if error.kind() == io::ErrorKind::NotFound {
                StagingError::PackageMissing(package_dir.to_path_buf())
            } else {
                StagingError::Io(error)
            }
        })?;
        if !metadata.is_dir() {
            return Err(StagingError::PackageNotDirectory(package_dir.to_path_buf()));
        }

        let staging_parent = self.update_dir.join("staging");
        fs::create_dir_all(&staging_parent)?;
        let id = STAGING_COUNTER.fetch_add(1, Ordering::Relaxed);
        let staging_dir = staging_parent.join(format!("package-{}-{id}", std::process::id()));
        copy_tree(package_dir, &staging_dir, &self.user_data_dir)?;

        Ok(StagedUpdate {
            layout: self.clone(),
            staging_dir,
            backup_dir: self.update_dir.join("backup"),
        })
    }

    /// Verifies and extracts an archive into disposable storage before copying its contents into
    /// the normal sibling staging area. User data is never part of the extraction tree.
    pub fn stage_verified_archive(
        &self,
        manifest: &ReleaseManifest,
        public_key: &[u8; 32],
        archive: &[u8],
        expected_version: &str,
        expected_channel: Channel,
    ) -> Result<StagedUpdate, StagingError> {
        manifest
            .verify_windows_x64(public_key, archive, expected_version, expected_channel)
            .map_err(StagingError::Verification)?;

        self.extract_and_stage(|extract_dir| unpack_archive(archive, extract_dir))
    }

    /// Verifies a streamed archive file on disk against `manifest` and extracts it into the
    /// sibling staging area without reading the whole archive into memory.
    pub fn stage_verified_archive_file(
        &self,
        manifest: &ReleaseManifest,
        public_key: &[u8; 32],
        archive_path: &Path,
        expected_version: &str,
        expected_channel: Channel,
    ) -> Result<StagedUpdate, StagingError> {
        manifest
            .verify_windows_x64_file(public_key, archive_path, expected_version, expected_channel)
            .map_err(StagingError::Verification)?;

        self.extract_and_stage(|extract_dir| unpack_archive_file(archive_path, extract_dir))
    }

    fn extract_and_stage<F>(&self, unpack: F) -> Result<StagedUpdate, StagingError>
    where
        F: FnOnce(&Path) -> Result<PathBuf, ArchiveError>,
    {
        fs::create_dir_all(&self.update_dir)?;
        let extract_dir = self.update_dir.join(format!(
            "extract-{}",
            STAGING_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let package_dir = match unpack(&extract_dir) {
            Ok(package_dir) => package_dir,
            Err(error) => {
                let _ = fs::remove_dir_all(&extract_dir);
                return Err(StagingError::Archive(error));
            }
        };
        let staged = self.stage_from_dir(&package_dir);
        let cleanup = fs::remove_dir_all(&extract_dir);
        match (staged, cleanup) {
            (Ok(staged), Ok(())) => Ok(staged),
            (Err(error), _) => Err(error),
            (Ok(_), Err(error)) => Err(StagingError::Io(error)),
        }
    }

    /// Restores the previous application when a process was interrupted after the old tree was
    /// moved aside but before the new tree was moved into place. Returns whether recovery ran.
    pub fn recover_interrupted(&self) -> Result<bool, StagingError> {
        if self.application_dir.exists() || !self.backup_dir().exists() {
            return Ok(false);
        }
        fs::rename(self.backup_dir(), &self.application_dir)?;
        Ok(true)
    }

    /// Removes what a finished update left in the update area; see [`clear_finished_update`].
    pub fn clear_finished_update(&self) -> Result<bool, StagingError> {
        clear_finished_update(&self.application_dir)
    }

    fn backup_dir(&self) -> PathBuf {
        self.update_dir.join("backup")
    }
}

/// Removes what a finished update leaves in the `.ostrog-update` area beside
/// `application_dir`: the previous application kept for rollback, and staging or extraction trees
/// of attempts that never committed. Does nothing while `application_dir` is missing, because
/// then the backup is the only copy (an interrupted swap, see
/// [`InstallLayout::recover_interrupted`]). Returns whether anything was removed.
///
/// Without this the backup of the first update stayed forever, every later commit failed with
/// `BackupAlreadyExists`, and each start downloaded and staged the update again (WP-02 2.9).
pub fn clear_finished_update(application_dir: &Path) -> Result<bool, StagingError> {
    if !application_dir.is_dir() {
        return Ok(false);
    }
    let Some(install_dir) = application_dir.parent() else {
        return Ok(false);
    };
    let update_dir = install_dir.join(".ostrog-update");
    if !update_dir.is_dir() {
        return Ok(false);
    }
    let mut removed = false;
    for entry in fs::read_dir(&update_dir)? {
        let path = entry?.path();
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if path.is_dir() && (name == "backup" || name == "staging" || name.starts_with("extract-"))
        {
            fs::remove_dir_all(&path)?;
            removed = true;
        }
    }
    Ok(removed)
}

/// A verified package copied outside the running application tree.
#[derive(Debug)]
pub struct StagedUpdate {
    layout: InstallLayout,
    staging_dir: PathBuf,
    backup_dir: PathBuf,
}

impl StagedUpdate {
    pub fn staging_dir(&self) -> &Path {
        &self.staging_dir
    }

    /// Atomically swaps the application directory and retains the old tree for rollback.
    pub fn commit(self) -> Result<CommittedUpdate, StagingError> {
        if self.backup_dir.exists() {
            return Err(StagingError::BackupAlreadyExists(self.backup_dir));
        }

        if self.layout.application_dir.exists() {
            fs::rename(&self.layout.application_dir, &self.backup_dir)?;
        }

        if let Err(error) = fs::rename(&self.staging_dir, &self.layout.application_dir) {
            if self.backup_dir.exists() {
                let _ = fs::rename(&self.backup_dir, &self.layout.application_dir);
            }
            return Err(StagingError::Io(error));
        }

        Ok(CommittedUpdate {
            layout: self.layout,
            backup_dir: self.backup_dir,
        })
    }

    pub fn discard(self) -> Result<(), StagingError> {
        if self.staging_dir.exists() {
            fs::remove_dir_all(self.staging_dir)?;
        }
        Ok(())
    }
}

/// A completed swap whose previous application tree can still be restored.
#[derive(Debug)]
pub struct CommittedUpdate {
    layout: InstallLayout,
    backup_dir: PathBuf,
}

impl CommittedUpdate {
    /// The new application started, so the previous tree is no longer needed for rollback.
    pub fn finish(self) -> Result<(), StagingError> {
        if self.backup_dir.exists() {
            fs::remove_dir_all(&self.backup_dir)?;
        }
        Ok(())
    }

    pub fn rollback(self) -> Result<(), StagingError> {
        if !self.backup_dir.exists() {
            return Err(StagingError::NoBackup(self.backup_dir));
        }
        if self.layout.application_dir.exists() {
            fs::remove_dir_all(&self.layout.application_dir)?;
        }
        fs::rename(self.backup_dir, self.layout.application_dir)?;
        Ok(())
    }
}

fn copy_tree(source: &Path, destination: &Path, user_data_dir: &Path) -> Result<(), StagingError> {
    if is_same_or_descendant(source, user_data_dir) {
        return Err(StagingError::SymlinkNotAllowed(source.to_path_buf()));
    }
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let metadata = fs::symlink_metadata(&source_path)?;
        if metadata.file_type().is_symlink() {
            return Err(StagingError::SymlinkNotAllowed(source_path));
        }
        if metadata.is_dir() {
            copy_tree(&source_path, &destination_path, user_data_dir)?;
        } else if metadata.is_file() {
            fs::copy(&source_path, &destination_path)?;
        } else {
            return Err(StagingError::SymlinkNotAllowed(source_path));
        }
    }
    Ok(())
}

fn is_same_or_descendant(candidate: &Path, parent: &Path) -> bool {
    candidate == parent || candidate.starts_with(parent)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use serde_json::json;
    use sha2::{Digest, Sha256};
    use std::{
        io::Cursor,
        time::{SystemTime, UNIX_EPOCH},
    };
    use zip::write::{SimpleFileOptions, ZipWriter};

    struct TempTree(PathBuf);

    impl TempTree {
        fn new(label: &str) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos();
            let path =
                std::env::temp_dir().join(format!("ostrog-{label}-{}-{nonce}", std::process::id()));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempTree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn write(path: &Path, contents: &[u8]) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, contents).unwrap();
    }

    fn signed_archive() -> (ReleaseManifest, SigningKey, Vec<u8>) {
        let archive = {
            let mut bytes = Cursor::new(Vec::new());
            let mut writer = ZipWriter::new(&mut bytes);
            writer
                .start_file(
                    "ostrog-windows-x64/ostrog.exe",
                    SimpleFileOptions::default(),
                )
                .unwrap();
            std::io::Write::write_all(&mut writer, b"new").unwrap();
            writer.finish().unwrap();
            bytes.into_inner()
        };
        let key = SigningKey::from_bytes(&[11; 32]);
        let hash = hex::encode(Sha256::digest(&archive));
        let url = "https://releases.example/ostrog-windows-x64.zip";
        let payload = crate::manifest_signing_payload(
            "1.2.3",
            Channel::Stable,
            crate::Platform::Windows,
            crate::Architecture::X86_64,
            url,
            &hash,
        );
        let manifest = json!({
            "schema": 1,
            "version": "1.2.3",
            "channel": "stable",
            "platform": "windows",
            "architecture": "x86_64",
            "archive_url": url,
            "archive_sha256": hash,
            "signature": hex::encode(key.sign(&payload).to_bytes()),
        });
        (
            ReleaseManifest::parse(&manifest.to_string()).unwrap(),
            key,
            archive,
        )
    }

    #[test]
    fn staging_commit_and_rollback_preserve_user_data() {
        let tree = TempTree::new("commit");
        let app = tree.path().join("app");
        let data = tree.path().join("saves");
        let package = tree.path().join("package");
        write(&app.join("ostrog.exe"), b"old");
        write(&data.join("world.save"), b"player data");
        write(&package.join("ostrog.exe"), b"new");

        let layout = InstallLayout::new(&app, &data).unwrap();
        let committed = layout.stage_from_dir(&package).unwrap().commit().unwrap();
        assert_eq!(fs::read(app.join("ostrog.exe")).unwrap(), b"new");
        assert_eq!(fs::read(data.join("world.save")).unwrap(), b"player data");

        committed.rollback().unwrap();
        assert_eq!(fs::read(app.join("ostrog.exe")).unwrap(), b"old");
        assert_eq!(fs::read(data.join("world.save")).unwrap(), b"player data");
    }

    #[test]
    fn verified_archive_stages_without_touching_user_data() {
        let tree = TempTree::new("archive");
        let app = tree.path().join("app");
        let data = tree.path().join("saves");
        write(&app.join("ostrog.exe"), b"old");
        write(&data.join("world.save"), b"player data");
        let (manifest, key, archive) = signed_archive();

        let layout = InstallLayout::new(&app, &data).unwrap();
        let committed = layout
            .stage_verified_archive(
                &manifest,
                &key.verifying_key().to_bytes(),
                &archive,
                "1.2.3",
                Channel::Stable,
            )
            .unwrap()
            .commit()
            .unwrap();

        assert_eq!(fs::read(app.join("ostrog.exe")).unwrap(), b"new");
        assert_eq!(fs::read(data.join("world.save")).unwrap(), b"player data");
        committed.rollback().unwrap();
        assert_eq!(fs::read(app.join("ostrog.exe")).unwrap(), b"old");
    }

    #[test]
    fn verified_archive_file_stages_from_disk_without_whole_archive_vec() {
        let tree = TempTree::new("archive-file");
        let app = tree.path().join("app");
        let data = tree.path().join("saves");
        let archive_path = tree.path().join("downloaded.zip");
        write(&app.join("ostrog.exe"), b"old");
        write(&data.join("world.save"), b"player data");
        let (manifest, key, archive) = signed_archive();
        write(&archive_path, &archive);

        let layout = InstallLayout::new(&app, &data).unwrap();
        let committed = layout
            .stage_verified_archive_file(
                &manifest,
                &key.verifying_key().to_bytes(),
                &archive_path,
                "1.2.3",
                Channel::Stable,
            )
            .unwrap()
            .commit()
            .unwrap();

        assert_eq!(fs::read(app.join("ostrog.exe")).unwrap(), b"new");
        assert_eq!(fs::read(data.join("world.save")).unwrap(), b"player data");
        committed.rollback().unwrap();
        assert_eq!(fs::read(app.join("ostrog.exe")).unwrap(), b"old");
    }

    #[test]
    fn interrupted_swap_recovers_previous_application() {
        let tree = TempTree::new("recover");
        let app = tree.path().join("app");
        let data = tree.path().join("saves");
        write(&app.join("ostrog.exe"), b"old");
        let layout = InstallLayout::new(&app, &data).unwrap();
        fs::create_dir_all(layout.update_dir()).unwrap();
        fs::rename(&app, layout.update_dir().join("backup")).unwrap();

        assert!(layout.recover_interrupted().unwrap());
        assert_eq!(fs::read(app.join("ostrog.exe")).unwrap(), b"old");
        assert!(!layout.update_dir().join("backup").exists());
    }

    #[test]
    fn a_second_update_commits_once_the_first_ones_leftovers_are_cleared() {
        let tree = TempTree::new("second-update");
        let app = tree.path().join("app");
        let data = tree.path().join("saves");
        write(&app.join("ostrog.exe"), b"0.1.0");
        write(&data.join("world.save"), b"player data");
        write(&tree.path().join("one/ostrog.exe"), b"0.1.1");
        write(&tree.path().join("two/ostrog.exe"), b"0.1.2");
        let layout = InstallLayout::new(&app, &data).unwrap();

        // The first update as a 0.1.0 launcher leaves it: the old tree kept as the backup.
        layout
            .stage_from_dir(tree.path().join("one"))
            .unwrap()
            .commit()
            .unwrap();
        // A failed attempt's staged tree is left behind too.
        layout.stage_from_dir(tree.path().join("two")).unwrap();
        assert!(matches!(
            layout
                .stage_from_dir(tree.path().join("two"))
                .unwrap()
                .commit(),
            Err(StagingError::BackupAlreadyExists(_))
        ));

        assert!(layout.clear_finished_update().unwrap());
        assert!(!layout.update_dir().join("backup").exists());
        assert!(!layout.update_dir().join("staging").exists());
        assert_eq!(fs::read(app.join("ostrog.exe")).unwrap(), b"0.1.1");

        let committed = layout
            .stage_from_dir(tree.path().join("two"))
            .unwrap()
            .commit()
            .unwrap();
        assert_eq!(fs::read(app.join("ostrog.exe")).unwrap(), b"0.1.2");
        committed.finish().unwrap();
        assert!(!layout.update_dir().join("backup").exists());
        assert_eq!(fs::read(data.join("world.save")).unwrap(), b"player data");
        layout.clear_finished_update().unwrap();
        assert!(!layout.clear_finished_update().unwrap());
        assert_eq!(fs::read(app.join("ostrog.exe")).unwrap(), b"0.1.2");
    }

    #[test]
    fn clearing_keeps_the_backup_while_it_is_the_only_application() {
        let tree = TempTree::new("clear-interrupted");
        let app = tree.path().join("app");
        let data = tree.path().join("saves");
        write(&app.join("ostrog.exe"), b"old");
        let layout = InstallLayout::new(&app, &data).unwrap();
        fs::create_dir_all(layout.update_dir()).unwrap();
        fs::rename(&app, layout.update_dir().join("backup")).unwrap();

        assert!(!layout.clear_finished_update().unwrap());
        assert!(layout.recover_interrupted().unwrap());
        assert_eq!(fs::read(app.join("ostrog.exe")).unwrap(), b"old");
    }

    #[test]
    fn user_data_inside_application_is_rejected() {
        let tree = TempTree::new("layout");
        let app = tree.path().join("app");
        assert_eq!(
            InstallLayout::new(&app, app.join("saves")),
            Err(LayoutError::ApplicationContainsUserData)
        );
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_package_content_is_rejected() {
        use std::os::unix::fs::symlink;

        let tree = TempTree::new("symlink");
        let app = tree.path().join("app");
        let data = tree.path().join("saves");
        let package = tree.path().join("package");
        write(&package.join("real.txt"), b"safe");
        symlink(&data, package.join("saves")).unwrap();

        let layout = InstallLayout::new(&app, &data).unwrap();
        assert!(matches!(
            layout.stage_from_dir(&package),
            Err(StagingError::SymlinkNotAllowed(_))
        ));
    }
}
