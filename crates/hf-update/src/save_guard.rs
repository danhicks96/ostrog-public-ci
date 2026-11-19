//! A small cross-process marker for save writes.
//!
//! The game holds [`SaveWriteLock`] while its background save commit is writing. A launcher checks
//! [`save_write_in_progress`] before replacing the application tree. The marker is deliberately in
//! user data, never in the application directory that updates replace.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

pub const SAVE_LOCK_NAME: &str = ".ostrog-save-writing";

#[derive(Debug, Eq, PartialEq)]
pub enum SaveLockError {
    UserDataMissing(PathBuf),
    AlreadyHeld(PathBuf),
    Io(String),
}

impl std::fmt::Display for SaveLockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for SaveLockError {}

/// Returns the marker path used to coordinate save commits and launcher updates.
pub fn save_lock_path(user_data_dir: impl AsRef<Path>) -> PathBuf {
    user_data_dir.as_ref().join(SAVE_LOCK_NAME)
}

/// Returns whether a game save is currently being committed.
pub fn save_write_in_progress(user_data_dir: impl AsRef<Path>) -> bool {
    let user_data_dir = user_data_dir.as_ref();
    if save_lock_path(user_data_dir).exists() {
        return true;
    }
    // Godot keeps the normal save ring in `user://saves`. Check one level of named save roots so
    // the launcher can be configured with the user-data root while the game locks its save dir.
    fs::read_dir(user_data_dir)
        .ok()
        .into_iter()
        .flatten()
        .any(|entry| {
            entry
                .ok()
                .is_some_and(|entry| save_lock_path(entry.path()).exists())
        })
}

/// Godot's per-user data folder for `project` (its `user://`), where the game keeps its saves and
/// takes its save lock (`user://saves`, persist.gd). Godot's own rule for an exported project
/// with no custom user dir: `%APPDATA%\Godot\app_userdata\<project>` on Windows,
/// `~/Library/Application Support/Godot/app_userdata/<project>` on macOS, and
/// `$XDG_DATA_HOME/godot/app_userdata/<project>` (else `~/.local/share/...`) elsewhere. None when
/// the variable it needs is unset (WP-02 slice 2.15: the launcher looked only in its own
/// `data/` and never saw the game's lock).
pub fn godot_user_dir(project: &str) -> Option<PathBuf> {
    godot_user_dir_for(std::env::consts::OS, |k| std::env::var_os(k), project)
}

/// `godot_user_dir` for a given OS and environment, so each platform's rule can be tested.
pub fn godot_user_dir_for(
    os: &str,
    var: impl Fn(&str) -> Option<std::ffi::OsString>,
    project: &str,
) -> Option<PathBuf> {
    let set = |k: &str| var(k).filter(|v| !v.is_empty()).map(PathBuf::from);
    let base = match os {
        "windows" => set("APPDATA")?.join("Godot"),
        "macos" => set("HOME")?
            .join("Library")
            .join("Application Support")
            .join("Godot"),
        _ => set("XDG_DATA_HOME")
            // The helper is also called with a simulated Unix OS by the Windows test suite.
            // `Path::is_absolute` uses the host's Windows rules there, where `/data/f` is not
            // absolute even though it is an absolute XDG path for the simulated Unix target.
            .filter(|p| p.is_absolute() || p.to_string_lossy().starts_with('/'))
            .or_else(|| set("HOME").map(|h| h.join(".local").join("share")))?
            .join("godot"),
    };
    Some(base.join("app_userdata").join(project))
}

/// An exclusive marker held for the lifetime of a save commit.
#[derive(Debug)]
pub struct SaveWriteLock {
    path: PathBuf,
}

impl SaveWriteLock {
    pub fn acquire(user_data_dir: impl AsRef<Path>) -> Result<Self, SaveLockError> {
        let user_data_dir = user_data_dir.as_ref();
        if !user_data_dir.is_dir() {
            return Err(SaveLockError::UserDataMissing(user_data_dir.to_path_buf()));
        }
        let path = save_lock_path(user_data_dir);
        let mut file = fs::OpenOptions::new();
        file.write(true).create_new(true);
        let file = file.open(&path).map_err(|error| {
            if error.kind() == io::ErrorKind::AlreadyExists {
                SaveLockError::AlreadyHeld(path.clone())
            } else {
                SaveLockError::Io(error.to_string())
            }
        })?;
        let _ = file.set_len(0);
        Ok(Self { path })
    }
}

impl Drop for SaveWriteLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lock_blocks_updates_until_the_save_commit_finishes() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!save_write_in_progress(dir.path()));
        let lock = SaveWriteLock::acquire(dir.path()).unwrap();
        assert!(save_write_in_progress(dir.path()));
        assert!(matches!(
            SaveWriteLock::acquire(dir.path()),
            Err(SaveLockError::AlreadyHeld(_))
        ));
        drop(lock);
        assert!(!save_write_in_progress(dir.path()));
    }

    #[test]
    fn a_missing_user_data_root_is_not_created_by_the_lock() {
        let dir = tempfile::tempdir().unwrap().path().join("missing");
        assert!(matches!(
            SaveWriteLock::acquire(&dir),
            Err(SaveLockError::UserDataMissing(_))
        ));
        assert!(!dir.exists());
    }

    #[test]
    fn the_launcher_sees_a_lock_in_the_normal_nested_save_directory() {
        let root = tempfile::tempdir().unwrap();
        let saves = root.path().join("saves");
        fs::create_dir(&saves).unwrap();
        let lock = SaveWriteLock::acquire(&saves).unwrap();
        assert!(save_write_in_progress(root.path()));
        drop(lock);
        assert!(!save_write_in_progress(root.path()));
    }
}
