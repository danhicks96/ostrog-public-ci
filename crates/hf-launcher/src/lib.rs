//! The friend-facing process boundary for a portable Ostrog install.
//!
//! This crate deliberately owns policy orchestration, not the game. It checks a public signed
//! manifest before fetching its signed archive, keeps saves/settings outside the replaceable
//! application tree, and starts the game only after recovery/update work is complete. Network
//! failures always fall back to the installed build; no GitHub credential is accepted or stored.

#[cfg(windows)]
pub mod window;

use hf_update::{
    decide, save_write_in_progress, unpack_archive_file, Channel, CheckConfig, CheckDecision,
    DownloadProgress, DownloadedArchive, FetchError, FetchResult, InstallLayout, PolicyError,
    ReleaseManifest, StagingError,
};
use serde::Deserialize;
use std::{
    ffi::OsString,
    fs, io,
    path::{Path, PathBuf},
    process::{Child, Command},
    sync::atomic::{AtomicU64, Ordering},
};

static EXTRACT_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Default public signed-manifest URL hosted on the project server (DEC-3).
pub const DEFAULT_MANIFEST_URL: &str = "https://updates.example.com/stable/windows-x64.json";

/// Default 32-byte Ed25519 release public key in hex (DEC-1).
pub const DEFAULT_PUBLIC_KEY_HEX: &str =
    "8b7bd93f1f380ead6e015c6f68d87a301fa4a21c55f29ecab1098d5957e4ae7f";

/// Embedded default `launcher.toml` used when the single launcher executable starts in an empty
/// folder for first-install bootstrap (WP-02 slice 2.6).
pub const EMBEDDED_DEFAULT_TOML: &str = r#"manifest_url = "https://updates.example.com/stable/windows-x64.json"
public_key = "8b7bd93f1f380ead6e015c6f68d87a301fa4a21c55f29ecab1098d5957e4ae7f"
current_version = "0.0.0"
channel = "stable"
timeout_seconds = 10
check_enabled = true
application_dir = "app"
user_data_dir = "data"
executable = "ostrog.exe"
"#;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LauncherSettings {
    pub manifest_url: String,
    pub public_key: [u8; 32],
    pub current_version: String,
    pub channel: Channel,
    pub check: CheckConfig,
    pub application_dir: PathBuf,
    pub user_data_dir: PathBuf,
    pub executable: PathBuf,
    /// Where the game itself keeps its saves and takes its save lock: Godot's per-user folder
    /// for the project (hf_update::godot_user_dir). Not a `launcher.toml` key (an older launcher
    /// would refuse a config with a key it does not know); `load` and `load_or_default` fill it
    /// from the environment, `from_toml` leaves it None (WP-02 slice 2.15).
    pub game_user_dir: Option<PathBuf>,
}

/// The Godot project name (`config/name` in godot/project.godot), which names its user folder.
pub const GODOT_PROJECT_NAME: &str = "Ostrog";

/// The project name before the rebrand (issue #2106). Saves and settings written under it are
/// moved to the new folder the first time the renamed game's launcher starts.
pub const LEGACY_GODOT_PROJECT_NAME: &str = "Hearthfall";

/// Moves the pre-rebrand Godot user folder to its new name so existing saves and settings keep
/// loading. Does nothing when the old folder is missing or the new one already exists (never
/// overwrites). Best effort: a failed rename leaves both folders as they were.
pub fn migrate_legacy_user_dir(new_dir: &Path, legacy_dir: &Path) -> bool {
    if new_dir.exists() || !legacy_dir.is_dir() {
        return false;
    }
    fs::rename(legacy_dir, new_dir).is_ok()
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSettings {
    manifest_url: String,
    public_key: String,
    current_version: String,
    channel: String,
    timeout_seconds: u32,
    check_enabled: bool,
    deferred_until: Option<u64>,
    application_dir: PathBuf,
    user_data_dir: PathBuf,
    executable: PathBuf,
}

impl LauncherSettings {
    pub fn from_toml(text: &str) -> Result<Self, ConfigError> {
        let raw: RawSettings =
            toml::from_str(text).map_err(|error| ConfigError::InvalidToml(error.to_string()))?;
        let public_key = hex::decode(&raw.public_key)
            .map_err(|_| ConfigError::InvalidPublicKey)?
            .try_into()
            .map_err(|_| ConfigError::InvalidPublicKey)?;
        let channel = raw
            .channel
            .parse()
            .map_err(|_| ConfigError::InvalidChannel(raw.channel.clone()))?;
        let check = CheckConfig {
            enabled: raw.check_enabled,
            timeout_seconds: raw.timeout_seconds,
            deferred_until: raw.deferred_until,
        };
        check.validate().map_err(ConfigError::InvalidPolicy)?;
        if is_absolute_on_any_supported_host(&raw.executable) {
            return Err(ConfigError::ExecutableMustBeRelative(raw.executable));
        }
        Ok(Self {
            manifest_url: raw.manifest_url,
            public_key,
            current_version: raw.current_version,
            channel,
            check,
            application_dir: raw.application_dir,
            user_data_dir: raw.user_data_dir,
            executable: raw.executable,
            game_user_dir: None,
        })
    }

    /// Whether the game is committing a save, in the launcher's `data/` or in the game's own
    /// per-user folder, where its saves really are (WP-02 slice 2.15, FIXLOG 2026-11-11: the
    /// launcher looked only in `data/` and never saw the game's marker).
    pub fn save_in_progress(&self) -> bool {
        save_write_in_progress(&self.user_data_dir)
            || self
                .game_user_dir
                .as_ref()
                .is_some_and(save_write_in_progress)
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        let text = fs::read_to_string(path).map_err(|error| ConfigError::Io {
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;
        let mut settings = Self::from_toml(&text)?;
        let base = path.parent().unwrap_or_else(|| Path::new("."));
        settings.resolve_relative_dirs(base);
        Ok(settings)
    }

    /// Loads `path` when it exists, or returns embedded default settings resolved relative to
    /// `path`'s parent directory so a single `Ostrog.exe` in an empty folder can bootstrap the
    /// install without a pre-existing `launcher.toml`.
    pub fn load_or_default(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        match fs::read_to_string(path) {
            Ok(text) => {
                let mut settings = Self::from_toml(&text)?;
                let base = path.parent().unwrap_or_else(|| Path::new("."));
                settings.resolve_relative_dirs(base);
                Ok(settings)
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let mut settings =
                    Self::from_toml(EMBEDDED_DEFAULT_TOML).expect("embedded default TOML is valid");
                let base = path.parent().unwrap_or_else(|| Path::new("."));
                settings.resolve_relative_dirs(base);
                Ok(settings)
            }
            Err(error) => Err(ConfigError::Io {
                path: path.to_path_buf(),
                message: error.to_string(),
            }),
        }
    }

    fn resolve_relative_dirs(&mut self, base: &Path) {
        if self.game_user_dir.is_none() {
            self.game_user_dir = hf_update::godot_user_dir(GODOT_PROJECT_NAME);
            if let (Some(new_dir), Some(legacy_dir)) = (
                self.game_user_dir.as_deref(),
                hf_update::godot_user_dir(LEGACY_GODOT_PROJECT_NAME),
            ) {
                migrate_legacy_user_dir(new_dir, &legacy_dir);
            }
        }
        if self.application_dir.is_relative() {
            self.application_dir = base.join(&self.application_dir);
        }
        if self.user_data_dir.is_relative() {
            self.user_data_dir = base.join(&self.user_data_dir);
        }
    }

    /// Returns the effective installed version: if `application_dir/VERSION` exists and is a valid
    /// `MAJOR.MINOR.PATCH` string, that version takes precedence over `current_version` in
    /// `launcher.toml` so subsequent launches after an update or bootstrap know the installed version.
    pub fn effective_current_version(&self) -> String {
        let version_path = self.application_dir.join("VERSION");
        if let Ok(raw) = fs::read_to_string(version_path) {
            let trimmed = raw.trim();
            if hf_update::validate_version(trimmed).is_ok() {
                return trimmed.to_owned();
            }
        }
        self.current_version.clone()
    }

    pub fn installed_executable_path(&self) -> PathBuf {
        self.application_dir.join(&self.executable)
    }

    pub fn has_installed_build(&self) -> bool {
        self.installed_executable_path().is_file()
    }
}

fn is_absolute_on_any_supported_host(path: &Path) -> bool {
    if path.is_absolute() {
        return true;
    }
    let text = path.as_os_str().to_string_lossy();
    text.starts_with("\\\\")
        || (text.len() >= 3
            && text.as_bytes()[0].is_ascii_alphabetic()
            && text.as_bytes()[1] == b':'
            && matches!(text.as_bytes()[2], b'/' | b'\\'))
}

#[derive(Debug, Eq, PartialEq)]
pub enum ConfigError {
    Io { path: PathBuf, message: String },
    InvalidToml(String),
    InvalidPublicKey,
    InvalidChannel(String),
    InvalidPolicy(PolicyError),
    ExecutableMustBeRelative(PathBuf),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for ConfigError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LaunchReason {
    Disabled,
    Deferred,
    CurrentVersion,
    Offline,
    TimedOut,
    Rejected,
    WrongChannel,
    SaveInProgress,
    UpdateFailed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LaunchPlan {
    StartCurrent { reason: LaunchReason },
    Update { version: String },
}

#[derive(Debug)]
pub enum LauncherError {
    Config(ConfigError),
    Layout(hf_update::LayoutError),
    Fetch(FetchError),
    Policy(PolicyError),
    Staging(StagingError),
    Launch(io::Error),
    NoInstalledBuild { reason: LaunchReason },
}

impl std::fmt::Display for LauncherError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for LauncherError {}

impl From<ConfigError> for LauncherError {
    fn from(error: ConfigError) -> Self {
        Self::Config(error)
    }
}

/// User-facing startup stages reported by the launcher (WP-02 status table).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LauncherStage {
    Starting,
    Checking,
    Offline,
    Downloading { version: String },
    Verifying,
    Installing,
    Launching,
    Failure { message: String },
}

impl LauncherStage {
    /// Returns the short stage label used in logs and screenshot filenames.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Starting => "starting",
            Self::Checking => "checking",
            Self::Offline => "offline",
            Self::Downloading { .. } => "downloading",
            Self::Verifying => "verifying",
            Self::Installing => "installing",
            Self::Launching => "launching",
            Self::Failure { .. } => "failure",
        }
    }

    /// The small caption under the status window's progress bar. Saves live in the game's
    /// per-user folder (%APPDATA%\Godot\app_userdata\Ostrog\), not in the launcher's data\:
    /// the caption said "Saves protected in data\" until FIXLOG 2026-11-13 (WP-02 2.15).
    pub fn footer(&self) -> String {
        format!("Stage: {}  •  Saves kept in AppData", self.name())
    }

    /// Formats the friend-facing status message specified by the WP-02 stage table.
    pub fn format_status(
        &self,
        has_installed_build: bool,
        bytes: u64,
        total: Option<u64>,
        elapsed_secs: f64,
    ) -> String {
        match self {
            Self::Starting => "Starting Ostrog...".to_owned(),
            Self::Checking => "Checking for a newer version...".to_owned(),
            Self::Offline => {
                if has_installed_build {
                    "Can't reach the update server. Starting your installed version.".to_owned()
                } else {
                    "Can't reach the server. Check your internet and try again.".to_owned()
                }
            }
            Self::Downloading { version } => {
                const MB: f64 = 1_048_576.0;
                let done_mb = (bytes as f64 / MB).round() as u64;
                match total.filter(|&t| t > 0) {
                    Some(total_bytes) => {
                        let total_mb = (total_bytes as f64 / MB).round().max(1.0) as u64;
                        let pct = ((bytes.saturating_mul(100)) / total_bytes).min(100);
                        let eta_suffix = if elapsed_secs >= 0.25 && bytes > 0 && bytes < total_bytes
                        {
                            let rate = bytes as f64 / elapsed_secs;
                            let rem = ((total_bytes - bytes) as f64 / rate).ceil() as u64;
                            format!(", about {rem} s left")
                        } else {
                            String::new()
                        };
                        format!(
                            "Downloading Ostrog {version}: {done_mb} of {total_mb} MB ({pct}%){eta_suffix}"
                        )
                    }
                    None => format!("Downloading Ostrog {version}: {done_mb} MB"),
                }
            }
            Self::Verifying => "Checking the download is genuine...".to_owned(),
            Self::Installing => "Installing... this takes a moment".to_owned(),
            Self::Launching => "Starting the game. A window will open shortly.".to_owned(),
            Self::Failure { message } => format!(
                "Something went wrong: {message}. Your saved games are safe. Details: data\\logs\\launcher.log"
            ),
        }
    }
}

/// Appends a timestamped status line to `<user_data_dir>/logs/launcher.log`.
pub fn append_launcher_log(user_data_dir: &Path, line: &str) -> io::Result<PathBuf> {
    use std::io::Write;
    let logs_dir = user_data_dir.join("logs");
    fs::create_dir_all(&logs_dir)?;
    let log_path = logs_dir.join("launcher.log");
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;
    writeln!(file, "[{ts}] {line}")?;
    Ok(log_path)
}

/// Receives live stage transitions and byte-progress updates during `run`.
pub trait ProgressSink {
    fn on_progress(&mut self, stage: LauncherStage, bytes: u64, total: Option<u64>);
}

struct LoggingProgressSink<'a, S: ProgressSink + ?Sized> {
    inner: &'a mut S,
    user_data_dir: &'a Path,
    has_installed_build: bool,
}

impl<S: ProgressSink + ?Sized> ProgressSink for LoggingProgressSink<'_, S> {
    fn on_progress(&mut self, stage: LauncherStage, bytes: u64, total: Option<u64>) {
        // Log stage transitions and final failure messages without spamming every 64 KiB chunk.
        if bytes == 0 || total.is_some_and(|t| bytes == t) {
            let status = stage.format_status(self.has_installed_build, bytes, total, 0.0);
            let _ = append_launcher_log(
                self.user_data_dir,
                &format!(
                    "stage={} bytes={bytes} total={total:?} text=\"{status}\"",
                    stage.name()
                ),
            );
        }
        self.inner.on_progress(stage, bytes, total);
    }
}

/// Default no-op progress sink used when no UI observer is attached.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoopProgressSink;

impl ProgressSink for NoopProgressSink {
    fn on_progress(&mut self, _stage: LauncherStage, _bytes: u64, _total: Option<u64>) {}
}

impl<F> ProgressSink for F
where
    F: FnMut(LauncherStage, u64, Option<u64>),
{
    fn on_progress(&mut self, stage: LauncherStage, bytes: u64, total: Option<u64>) {
        self(stage, bytes, total);
    }
}

/// The network and process boundary used by the launcher.
///
/// Keeping both boundaries injectable lets tests exercise update failure recovery without a real
/// release server or an uncontrolled game process. The production implementation below remains
/// anonymous HTTPS plus the installed executable.
pub trait LauncherRuntime {
    fn fetch_manifest(&self, url: &str, config: CheckConfig)
        -> Result<ReleaseManifest, FetchError>;

    fn fetch_archive(
        &self,
        url: &str,
        config: CheckConfig,
        on_progress: &mut dyn FnMut(DownloadProgress),
    ) -> Result<DownloadedArchive, FetchError>;

    fn spawn(&self, executable: &Path, args: &[OsString]) -> io::Result<Child>;
}

struct SystemRuntime;

impl LauncherRuntime for SystemRuntime {
    fn fetch_manifest(
        &self,
        url: &str,
        config: CheckConfig,
    ) -> Result<ReleaseManifest, FetchError> {
        hf_update::fetch_manifest(url, config)
    }

    fn fetch_archive(
        &self,
        url: &str,
        config: CheckConfig,
        on_progress: &mut dyn FnMut(DownloadProgress),
    ) -> Result<DownloadedArchive, FetchError> {
        hf_update::fetch_archive(url, config, on_progress)
    }

    fn spawn(&self, executable: &Path, args: &[OsString]) -> io::Result<Child> {
        Command::new(executable).args(args).spawn()
    }
}

/// Classifies a verified or failed startup check without doing I/O or launching a process.
pub fn classify(
    settings: &LauncherSettings,
    now: u64,
    fetched: FetchResult<'_>,
) -> Result<LaunchPlan, LauncherError> {
    classify_version(settings, &settings.current_version, now, fetched)
}

fn classify_version(
    settings: &LauncherSettings,
    installed_version: &str,
    now: u64,
    fetched: FetchResult<'_>,
) -> Result<LaunchPlan, LauncherError> {
    let decision = decide(
        settings.check,
        now,
        installed_version,
        settings.channel,
        fetched,
    )
    .map_err(LauncherError::Policy)?;
    Ok(match decision {
        CheckDecision::Disabled => LaunchPlan::StartCurrent {
            reason: LaunchReason::Disabled,
        },
        CheckDecision::Deferred => LaunchPlan::StartCurrent {
            reason: LaunchReason::Deferred,
        },
        CheckDecision::CurrentVersion => LaunchPlan::StartCurrent {
            reason: LaunchReason::CurrentVersion,
        },
        CheckDecision::Offer { version, .. } => LaunchPlan::Update { version },
        CheckDecision::LaunchCurrent { reason } => LaunchPlan::StartCurrent {
            reason: match reason {
                hf_update::FallbackReason::Offline => LaunchReason::Offline,
                hf_update::FallbackReason::TimedOut => LaunchReason::TimedOut,
                hf_update::FallbackReason::Rejected => LaunchReason::Rejected,
                hf_update::FallbackReason::WrongChannel => LaunchReason::WrongChannel,
            },
        },
    })
}

#[derive(Debug)]
pub enum LaunchOutcome {
    StartedCurrent { child: Child, reason: LaunchReason },
    UpdatedAndStarted { child: Child, version: String },
}

/// Runs one startup check, safely stages an update if offered, and starts the installed game.
pub fn run(
    settings: &LauncherSettings,
    now: u64,
    args: impl IntoIterator<Item = OsString>,
) -> Result<LaunchOutcome, LauncherError> {
    let mut sink = NoopProgressSink;
    run_with_progress(settings, now, args, &mut sink)
}

/// Runs one startup check while reporting stage transitions and byte progress to `sink`.
pub fn run_with_progress<S: ProgressSink + ?Sized>(
    settings: &LauncherSettings,
    now: u64,
    args: impl IntoIterator<Item = OsString>,
    sink: &mut S,
) -> Result<LaunchOutcome, LauncherError> {
    let runtime = SystemRuntime;
    run_with_runtime_and_progress(settings, now, args, &runtime, sink)
}

/// Runs startup with an explicit network/process boundary for deterministic callers and tests.
pub fn run_with_runtime<R: LauncherRuntime + ?Sized>(
    settings: &LauncherSettings,
    now: u64,
    args: impl IntoIterator<Item = OsString>,
    runtime: &R,
) -> Result<LaunchOutcome, LauncherError> {
    let mut sink = NoopProgressSink;
    run_with_runtime_and_progress(settings, now, args, runtime, &mut sink)
}

/// Runs startup with an explicit network/process boundary and a `ProgressSink`.
pub fn run_with_runtime_and_progress<R, S>(
    settings: &LauncherSettings,
    now: u64,
    args: impl IntoIterator<Item = OsString>,
    runtime: &R,
    sink: &mut S,
) -> Result<LaunchOutcome, LauncherError>
where
    R: LauncherRuntime + ?Sized,
    S: ProgressSink + ?Sized,
{
    let layout = match InstallLayout::new(&settings.application_dir, &settings.user_data_dir) {
        Ok(layout) => layout,
        Err(error) => {
            sink.on_progress(LauncherStage::Starting, 0, None);
            sink.on_progress(
                LauncherStage::Failure {
                    message: format!("invalid install layout: {error}"),
                },
                0,
                None,
            );
            return Err(LauncherError::Layout(error));
        }
    };
    let has_installed_build = settings.has_installed_build();
    let mut logging_sink = LoggingProgressSink {
        inner: sink,
        user_data_dir: &settings.user_data_dir,
        has_installed_build,
    };
    let sink = &mut logging_sink;
    sink.on_progress(LauncherStage::Starting, 0, None);
    let args: Vec<OsString> = args.into_iter().collect();
    if let Err(error) = layout.recover_interrupted() {
        sink.on_progress(
            LauncherStage::Failure {
                message: format!("could not recover interrupted update: {error}"),
            },
            0,
            None,
        );
        return Err(LauncherError::Staging(error));
    }
    // A finished update's backup and staged trees would make every later update fail at the swap
    // (BackupAlreadyExists) and pile up on disk. Failing to remove them must not stop the game.
    if let Err(error) = layout.clear_finished_update() {
        let _ = append_launcher_log(
            &settings.user_data_dir,
            &format!("could not clear the last update's leftovers: {error}"),
        );
    }

    let installed_version = settings.effective_current_version();

    let mut update: Option<(hf_update::ReleaseManifest, String)> = None;
    let plan = if settings.save_in_progress() {
        LaunchPlan::StartCurrent {
            reason: LaunchReason::SaveInProgress,
        }
    } else if !settings.check.enabled && has_installed_build {
        match classify_version(settings, &installed_version, now, FetchResult::Offline) {
            Ok(plan) => plan,
            Err(error) => {
                sink.on_progress(
                    LauncherStage::Failure {
                        message: format!("invalid update policy: {error}"),
                    },
                    0,
                    None,
                );
                return Err(error);
            }
        }
    } else {
        sink.on_progress(LauncherStage::Checking, 0, None);
        match runtime.fetch_manifest(&settings.manifest_url, settings.check) {
            Ok(manifest) => {
                let verified = manifest
                    .verify_windows_x64_target(&settings.public_key, settings.channel)
                    .is_ok();
                if !verified {
                    LaunchPlan::StartCurrent {
                        reason: LaunchReason::Rejected,
                    }
                } else if !has_installed_build {
                    // First install into an empty application directory: always fetch the
                    // signed manifest's build on the configured channel.
                    let version = manifest.version.clone();
                    update = Some((manifest, version.clone()));
                    LaunchPlan::Update { version }
                } else {
                    let plan = match classify_version(
                        settings,
                        &installed_version,
                        now,
                        FetchResult::Verified(&manifest),
                    ) {
                        Ok(plan) => plan,
                        Err(error) => {
                            sink.on_progress(
                                LauncherStage::Failure {
                                    message: format!("invalid update policy: {error}"),
                                },
                                0,
                                None,
                            );
                            return Err(error);
                        }
                    };
                    if let LaunchPlan::Update { version } = &plan {
                        update = Some((manifest, version.clone()));
                    }
                    plan
                }
            }
            Err(FetchError::Transport(_)) | Err(FetchError::HttpStatus(_)) => {
                sink.on_progress(LauncherStage::Offline, 0, None);
                LaunchPlan::StartCurrent {
                    reason: LaunchReason::Offline,
                }
            }
            Err(_) => LaunchPlan::StartCurrent {
                reason: LaunchReason::Rejected,
            },
        }
    };

    match plan {
        LaunchPlan::StartCurrent { reason } => {
            start_current(settings, &args, runtime, sink, reason)
        }
        LaunchPlan::Update { version } => {
            let (manifest, planned_version) = update.expect("update plan retains its manifest");
            debug_assert_eq!(planned_version, version);
            if settings.save_in_progress() {
                return start_current(settings, &args, runtime, sink, LaunchReason::SaveInProgress);
            }

            sink.on_progress(
                LauncherStage::Downloading {
                    version: planned_version.clone(),
                },
                0,
                None,
            );
            let download_version = planned_version.clone();
            let archive_result =
                runtime.fetch_archive(&manifest.archive_url, settings.check, &mut |progress| {
                    sink.on_progress(
                        LauncherStage::Downloading {
                            version: download_version.clone(),
                        },
                        progress.bytes_downloaded,
                        progress.total_bytes,
                    );
                });
            let archive = match archive_result {
                Ok(archive) => archive,
                Err(error) => {
                    return fallback_or_fail_update(
                        settings,
                        &args,
                        runtime,
                        sink,
                        format!("could not download update {planned_version}: {error}"),
                    );
                }
            };

            sink.on_progress(LauncherStage::Verifying, 0, None);
            if let Err(error) = manifest.verify_windows_x64_file(
                &settings.public_key,
                archive.path(),
                &planned_version,
                settings.channel,
            ) {
                return fallback_or_fail_update(
                    settings,
                    &args,
                    runtime,
                    sink,
                    format!("update verification failed: {error}"),
                );
            }

            sink.on_progress(LauncherStage::Installing, 0, None);
            let staged = match unpack_and_stage(&layout, archive.path(), &planned_version) {
                Ok(staged) => staged,
                Err(error) => {
                    return fallback_or_fail_update(
                        settings,
                        &args,
                        runtime,
                        sink,
                        format!("could not stage update {planned_version}: {error}"),
                    );
                }
            };
            let committed = match staged.commit() {
                Ok(committed) => committed,
                Err(error) => {
                    return fallback_or_fail_update(
                        settings,
                        &args,
                        runtime,
                        sink,
                        format!("could not commit update {planned_version}: {error}"),
                    );
                }
            };

            sink.on_progress(LauncherStage::Launching, 0, None);
            if let Err(error) = fs::create_dir_all(&settings.user_data_dir) {
                sink.on_progress(
                    LauncherStage::Failure {
                        message: format!("could not create user data directory: {error}"),
                    },
                    0,
                    None,
                );
                return Err(LauncherError::Launch(error));
            }
            match runtime.spawn(&settings.installed_executable_path(), &args) {
                Ok(child) => {
                    // The next start clears it if this fails.
                    let _ = committed.finish();
                    Ok(LaunchOutcome::UpdatedAndStarted {
                        child,
                        version: planned_version,
                    })
                }
                Err(launch_error) => {
                    if has_installed_build {
                        if let Err(rollback_error) = committed.rollback() {
                            sink.on_progress(
                                LauncherStage::Failure {
                                    message: format!(
                                        "update launch failed and rollback could not complete: {rollback_error}"
                                    ),
                                },
                                0,
                                None,
                            );
                            return Err(LauncherError::Staging(rollback_error));
                        }
                        match runtime.spawn(&settings.installed_executable_path(), &args) {
                            Ok(child) => Ok(LaunchOutcome::StartedCurrent {
                                child,
                                reason: LaunchReason::UpdateFailed,
                            }),
                            Err(fallback_error) => {
                                sink.on_progress(
                                    LauncherStage::Failure {
                                        message: format!(
                                            "could not start installed build after rollback: {fallback_error}"
                                        ),
                                    },
                                    0,
                                    None,
                                );
                                Err(LauncherError::Launch(fallback_error))
                            }
                        }
                    } else {
                        sink.on_progress(
                            LauncherStage::Failure {
                                message: format!("could not start Ostrog: {launch_error}"),
                            },
                            0,
                            None,
                        );
                        Err(LauncherError::Launch(launch_error))
                    }
                }
            }
        }
    }
}

fn unpack_and_stage(
    layout: &InstallLayout,
    archive_path: &Path,
    version: &str,
) -> Result<hf_update::StagedUpdate, StagingError> {
    fs::create_dir_all(layout.update_dir())?;
    let extract_dir = layout.update_dir().join(format!(
        "extract-launcher-{}-{}",
        std::process::id(),
        EXTRACT_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let package_dir = match unpack_archive_file(archive_path, &extract_dir) {
        Ok(package_dir) => package_dir,
        Err(error) => {
            let _ = fs::remove_dir_all(&extract_dir);
            return Err(StagingError::Archive(error));
        }
    };
    let staged = layout.stage_from_dir(&package_dir);
    let cleanup = fs::remove_dir_all(&extract_dir);
    match (staged, cleanup) {
        (Ok(staged), Ok(())) => {
            fs::write(staged.staging_dir().join("VERSION"), format!("{version}\n"))?;
            Ok(staged)
        }
        (Err(error), _) => Err(error),
        (Ok(_), Err(error)) => Err(StagingError::Io(error)),
    }
}

fn fallback_or_fail_update<R, S>(
    settings: &LauncherSettings,
    args: &[OsString],
    runtime: &R,
    sink: &mut S,
    failure_message: String,
) -> Result<LaunchOutcome, LauncherError>
where
    R: LauncherRuntime + ?Sized,
    S: ProgressSink + ?Sized,
{
    if settings.has_installed_build() {
        start_current(settings, args, runtime, sink, LaunchReason::UpdateFailed)
    } else {
        sink.on_progress(
            LauncherStage::Failure {
                message: failure_message,
            },
            0,
            None,
        );
        Err(LauncherError::NoInstalledBuild {
            reason: LaunchReason::UpdateFailed,
        })
    }
}

fn start_current<R, S>(
    settings: &LauncherSettings,
    args: &[OsString],
    runtime: &R,
    sink: &mut S,
    reason: LaunchReason,
) -> Result<LaunchOutcome, LauncherError>
where
    R: LauncherRuntime + ?Sized,
    S: ProgressSink + ?Sized,
{
    if !settings.has_installed_build() {
        sink.on_progress(
            LauncherStage::Failure {
                message: format!("no installed build is available ({reason:?})"),
            },
            0,
            None,
        );
        return Err(LauncherError::NoInstalledBuild { reason });
    }
    sink.on_progress(LauncherStage::Launching, 0, None);
    if let Err(error) = fs::create_dir_all(&settings.user_data_dir) {
        sink.on_progress(
            LauncherStage::Failure {
                message: format!("could not create user data directory: {error}"),
            },
            0,
            None,
        );
        return Err(LauncherError::Launch(error));
    }
    match runtime.spawn(&settings.installed_executable_path(), args) {
        Ok(child) => Ok(LaunchOutcome::StartedCurrent { child, reason }),
        Err(error) => {
            sink.on_progress(
                LauncherStage::Failure {
                    message: format!("could not start Ostrog: {error}"),
                },
                0,
                None,
            );
            Err(LauncherError::Launch(error))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    #[test]
    fn the_status_window_never_tells_a_friend_their_saves_are_in_data() {
        // FIXLOG 2026-11-13: the caption pointed at data\, where no save is ever written.
        let stages = [
            LauncherStage::Starting,
            LauncherStage::Checking,
            LauncherStage::Offline,
            LauncherStage::Verifying,
            LauncherStage::Installing,
            LauncherStage::Launching,
        ];
        for stage in stages {
            let footer = stage.footer();
            assert!(
                footer.starts_with(&format!("Stage: {}", stage.name())),
                "{footer}"
            );
            assert!(footer.contains("AppData"), "{footer}");
            assert!(!footer.contains("data\\"), "{footer}");
        }
    }
    use serde_json::json;
    use sha2::{Digest, Sha256};
    use std::{
        cell::Cell,
        fs,
        io::{Cursor, Write},
        time::{SystemTime, UNIX_EPOCH},
    };
    use zip::write::{SimpleFileOptions, ZipWriter};

    fn settings() -> LauncherSettings {
        LauncherSettings {
            manifest_url: "https://releases.example/manifest.json".into(),
            public_key: [7; 32],
            current_version: "1.2.3".into(),
            channel: Channel::Stable,
            check: CheckConfig {
                enabled: true,
                timeout_seconds: 10,
                deferred_until: None,
            },
            application_dir: "app".into(),
            user_data_dir: "data".into(),
            executable: "ostrog.exe".into(),
            game_user_dir: None,
        }
    }

    struct TempTree(PathBuf);

    impl TempTree {
        fn new(label: &str) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "ostrog-launcher-{label}-{}-{nonce}",
                std::process::id()
            ));
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

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct ProgressEvent {
        stage: LauncherStage,
        bytes: u64,
        total: Option<u64>,
    }

    #[derive(Default)]
    struct RecordingSink {
        events: Vec<ProgressEvent>,
    }

    impl ProgressSink for RecordingSink {
        fn on_progress(&mut self, stage: LauncherStage, bytes: u64, total: Option<u64>) {
            self.events.push(ProgressEvent {
                stage,
                bytes,
                total,
            });
        }
    }

    struct TestRuntime {
        manifest: ReleaseManifest,
        manifest_error: Option<String>,
        archive: Vec<u8>,
        archive_chunks: Vec<u64>,
        archive_error: Option<String>,
        launch_failures: Cell<usize>,
        spawn_calls: Cell<usize>,
    }

    impl TestRuntime {
        fn with_manifest(manifest: ReleaseManifest, archive: Vec<u8>) -> Self {
            let total = archive.len() as u64;
            let mid = (total / 2).max(1);
            Self {
                manifest,
                manifest_error: None,
                archive,
                archive_chunks: vec![mid, total],
                archive_error: None,
                launch_failures: Cell::new(0),
                spawn_calls: Cell::new(0),
            }
        }
    }

    impl LauncherRuntime for TestRuntime {
        fn fetch_manifest(
            &self,
            _url: &str,
            _config: CheckConfig,
        ) -> Result<ReleaseManifest, FetchError> {
            match &self.manifest_error {
                Some(message) => Err(FetchError::Transport(message.clone())),
                None => Ok(self.manifest.clone()),
            }
        }

        fn fetch_archive(
            &self,
            _url: &str,
            _config: CheckConfig,
            on_progress: &mut dyn FnMut(DownloadProgress),
        ) -> Result<DownloadedArchive, FetchError> {
            let total = Some(self.archive.len() as u64);
            match &self.archive_error {
                Some(message) => {
                    if let Some(&first_chunk) = self.archive_chunks.first() {
                        on_progress(DownloadProgress {
                            bytes_downloaded: first_chunk,
                            total_bytes: total,
                        });
                    }
                    Err(FetchError::Transport(message.clone()))
                }
                None => {
                    for &bytes_downloaded in &self.archive_chunks {
                        on_progress(DownloadProgress {
                            bytes_downloaded,
                            total_bytes: total,
                        });
                    }
                    DownloadedArchive::from_bytes(&self.archive)
                }
            }
        }

        fn spawn(&self, _executable: &Path, _args: &[OsString]) -> io::Result<Child> {
            let call = self.spawn_calls.get();
            self.spawn_calls.set(call + 1);
            if call < self.launch_failures.get() {
                return Err(io::Error::other("test launch failure"));
            }
            Command::new(std::env::current_exe()?).arg("--help").spawn()
        }
    }

    fn signed_archive() -> (ReleaseManifest, Vec<u8>, [u8; 32]) {
        let mut bytes = Cursor::new(Vec::new());
        {
            let mut writer = ZipWriter::new(&mut bytes);
            writer
                .start_file(
                    "ostrog-windows-x64/ostrog.exe",
                    SimpleFileOptions::default(),
                )
                .unwrap();
            writer.write_all(b"new").unwrap();
            writer.finish().unwrap();
        }
        let archive = bytes.into_inner();
        let key = SigningKey::from_bytes(&[7; 32]);
        let archive_hash = hex::encode(Sha256::digest(&archive));
        let archive_url = "https://releases.example/ostrog-windows-x64.zip";
        let payload = hf_update::manifest_signing_payload(
            "1.2.4",
            Channel::Stable,
            hf_update::Platform::Windows,
            hf_update::Architecture::X86_64,
            archive_url,
            &archive_hash,
        );
        let manifest = json!({
            "schema": 1,
            "version": "1.2.4",
            "channel": "stable",
            "platform": "windows",
            "architecture": "x86_64",
            "archive_url": archive_url,
            "archive_sha256": archive_hash,
            "signature": hex::encode(key.sign(&payload).to_bytes()),
        });
        (
            ReleaseManifest::parse(&manifest.to_string()).unwrap(),
            archive,
            key.verifying_key().to_bytes(),
        )
    }

    #[test]
    fn offline_startup_is_allowed_to_launch_the_installed_build() {
        assert_eq!(
            classify(&settings(), 100, FetchResult::Offline).unwrap(),
            LaunchPlan::StartCurrent {
                reason: LaunchReason::Offline
            }
        );
    }

    #[test]
    fn update_download_failure_launches_installed_build_and_preserves_data() {
        let tree = TempTree::new("download-failure");
        let app = tree.path().join("app");
        let data = tree.path().join("data");
        fs::create_dir_all(&app).unwrap();
        fs::create_dir_all(&data).unwrap();
        fs::write(app.join("ostrog.exe"), b"old").unwrap();
        fs::write(data.join("world.save"), b"player data").unwrap();

        let (manifest, archive, public_key) = signed_archive();
        let mut runtime = TestRuntime::with_manifest(manifest, archive);
        runtime.archive_error = Some("download failed".into());
        let mut launch_settings = settings();
        launch_settings.application_dir = app.clone();
        launch_settings.user_data_dir = data.clone();
        launch_settings.public_key = public_key;

        let outcome = run_with_runtime(&launch_settings, 100, Vec::new(), &runtime).unwrap();
        assert!(matches!(
            outcome,
            LaunchOutcome::StartedCurrent {
                reason: LaunchReason::UpdateFailed,
                ..
            }
        ));
        assert_eq!(runtime.spawn_calls.get(), 1);
        assert_eq!(fs::read(app.join("ostrog.exe")).unwrap(), b"old");
        assert_eq!(fs::read(data.join("world.save")).unwrap(), b"player data");
    }

    #[test]
    fn save_write_marker_defers_update_and_preserves_app_and_data() {
        let tree = TempTree::new("save-write");
        let app = tree.path().join("app");
        let data = tree.path().join("data");
        fs::create_dir_all(&app).unwrap();
        fs::create_dir_all(&data).unwrap();
        fs::write(app.join("ostrog.exe"), b"old").unwrap();
        fs::write(data.join("world.save"), b"player data").unwrap();

        let (manifest, archive, public_key) = signed_archive();
        let runtime = TestRuntime::with_manifest(manifest, archive);
        let mut launch_settings = settings();
        launch_settings.application_dir = app.clone();
        launch_settings.user_data_dir = data.clone();
        launch_settings.public_key = public_key;

        let lock = hf_update::SaveWriteLock::acquire(&data).unwrap();
        let outcome = run_with_runtime(&launch_settings, 100, Vec::new(), &runtime).unwrap();
        assert!(matches!(
            outcome,
            LaunchOutcome::StartedCurrent {
                reason: LaunchReason::SaveInProgress,
                ..
            }
        ));
        assert_eq!(runtime.spawn_calls.get(), 1);
        assert_eq!(fs::read(app.join("ostrog.exe")).unwrap(), b"old");
        assert_eq!(fs::read(data.join("world.save")).unwrap(), b"player data");
        assert!(!tree.path().join(".ostrog-update/backup").exists());
        drop(lock);
    }

    /// WP-02 slice 2.15: the game takes its save lock beside its save slots in Godot's per-user
    /// folder (`user://saves`), not in the launcher's `data/`; the launcher must see it there.
    #[test]
    fn a_save_lock_in_the_games_own_user_folder_defers_the_update() {
        let tree = TempTree::new("game-user-dir");
        let app = tree.path().join("app");
        let data = tree.path().join("data");
        let godot = tree.path().join("godot/app_userdata/Ostrog");
        let saves = godot.join("saves");
        fs::create_dir_all(&app).unwrap();
        fs::create_dir_all(&data).unwrap();
        fs::create_dir_all(saves.join("slot0.hfsave")).unwrap();
        fs::write(app.join("ostrog.exe"), b"old").unwrap();

        let (manifest, archive, public_key) = signed_archive();
        let runtime = TestRuntime::with_manifest(manifest, archive);
        let mut launch_settings = settings();
        launch_settings.application_dir = app.clone();
        launch_settings.user_data_dir = data.clone();
        launch_settings.public_key = public_key;
        launch_settings.game_user_dir = Some(godot.clone());

        // As hf-godot's save_to does: the lock in the parent of the slot being written.
        let lock = hf_update::SaveWriteLock::acquire(&saves).unwrap();
        assert!(!save_write_in_progress(&data), "data/ alone never sees it");
        assert!(launch_settings.save_in_progress());
        let outcome = run_with_runtime(&launch_settings, 100, Vec::new(), &runtime).unwrap();
        assert!(matches!(
            outcome,
            LaunchOutcome::StartedCurrent {
                reason: LaunchReason::SaveInProgress,
                ..
            }
        ));
        assert_eq!(fs::read(app.join("ostrog.exe")).unwrap(), b"old");
        drop(lock);
        assert!(!launch_settings.save_in_progress());
    }

    #[test]
    fn legacy_user_dir_moves_once_and_never_overwrites() {
        let tree = TempTree::new("legacy-user-dir");
        let old = tree.path().join("Hearthfall");
        let new = tree.path().join("Ostrog");
        fs::create_dir_all(old.join("saves")).unwrap();
        fs::write(old.join("saves/a.hfsave"), b"save").unwrap();
        assert!(migrate_legacy_user_dir(&new, &old));
        assert_eq!(fs::read(new.join("saves/a.hfsave")).unwrap(), b"save");
        assert!(!old.exists());
        fs::create_dir_all(&old).unwrap();
        assert!(
            !migrate_legacy_user_dir(&new, &old),
            "must not overwrite an existing folder"
        );
        assert!(old.exists());
    }

    #[test]
    fn godot_user_dir_follows_godots_rule_on_each_platform() {
        use std::ffi::OsString;
        let env = |pairs: &'static [(&'static str, &'static str)]| {
            move |k: &str| {
                pairs
                    .iter()
                    .find(|(n, _)| *n == k)
                    .map(|(_, v)| OsString::from(*v))
            }
        };
        let win = hf_update::godot_user_dir_for(
            "windows",
            env(&[("APPDATA", "C:\\Users\\f\\AppData\\Roaming")]),
            "Ostrog",
        )
        .unwrap();
        assert!(win.ends_with(Path::new("Godot").join("app_userdata").join("Ostrog")));
        assert!(win.starts_with("C:\\Users\\f\\AppData\\Roaming"));
        let linux =
            hf_update::godot_user_dir_for("linux", env(&[("HOME", "/home/user")]), "Ostrog");
        assert_eq!(
            linux,
            Some(PathBuf::from(
                "/home/user/.local/share/godot/app_userdata/Ostrog"
            ))
        );
        let xdg = hf_update::godot_user_dir_for(
            "linux",
            env(&[("HOME", "/home/user"), ("XDG_DATA_HOME", "/data/f")]),
            "Ostrog",
        );
        assert_eq!(
            xdg,
            Some(PathBuf::from("/data/f/godot/app_userdata/Ostrog"))
        );
        let mac = hf_update::godot_user_dir_for("macos", env(&[("HOME", "/Users/user")]), "Ostrog");
        assert_eq!(
            mac,
            Some(PathBuf::from(
                "/Users/user/Library/Application Support/Godot/app_userdata/Ostrog"
            ))
        );
        assert_eq!(
            hf_update::godot_user_dir_for("windows", env(&[]), "Ostrog"),
            None
        );
    }

    #[test]
    fn post_update_launch_failure_rolls_back_then_starts_installed_build() {
        let tree = TempTree::new("launch-failure");
        let app = tree.path().join("app");
        let data = tree.path().join("data");
        fs::create_dir_all(&app).unwrap();
        fs::create_dir_all(&data).unwrap();
        fs::write(app.join("ostrog.exe"), b"old").unwrap();
        fs::write(data.join("world.save"), b"player data").unwrap();

        let (manifest, archive, public_key) = signed_archive();
        let runtime = TestRuntime {
            launch_failures: Cell::new(1),
            ..TestRuntime::with_manifest(manifest, archive)
        };
        let mut launch_settings = settings();
        launch_settings.application_dir = app.clone();
        launch_settings.user_data_dir = data.clone();
        launch_settings.public_key = public_key;

        let outcome = run_with_runtime(&launch_settings, 100, Vec::new(), &runtime).unwrap();
        assert!(matches!(
            outcome,
            LaunchOutcome::StartedCurrent {
                reason: LaunchReason::UpdateFailed,
                ..
            }
        ));
        assert_eq!(runtime.spawn_calls.get(), 2);
        assert_eq!(fs::read(app.join("ostrog.exe")).unwrap(), b"old");
        assert_eq!(fs::read(data.join("world.save")).unwrap(), b"player data");
        assert!(!tree.path().join(".ostrog-update/backup").exists());
    }

    #[test]
    fn an_update_after_an_earlier_one_clears_its_leftovers_and_installs() {
        let tree = TempTree::new("second-update");
        let app = tree.path().join("app");
        let data = tree.path().join("data");
        fs::create_dir_all(&data).unwrap();
        fs::write(data.join("world.save"), b"player data").unwrap();
        fs::create_dir_all(&app).unwrap();
        fs::write(app.join("ostrog.exe"), b"old").unwrap();
        // What an earlier update left with a launcher that never removed it (WP-02 2.9): the
        // tree before that update, and a staged tree from an attempt that could not commit.
        let update_dir = tree.path().join(".ostrog-update");
        fs::create_dir_all(update_dir.join("backup")).unwrap();
        fs::write(update_dir.join("backup/ostrog.exe"), b"older").unwrap();
        fs::create_dir_all(update_dir.join("staging/package-1-1")).unwrap();
        fs::write(update_dir.join("staging/package-1-1/ostrog.exe"), b"new").unwrap();

        let (manifest, archive, public_key) = signed_archive();
        let runtime = TestRuntime::with_manifest(manifest, archive);
        let mut launch_settings = settings();
        launch_settings.application_dir = app.clone();
        launch_settings.user_data_dir = data.clone();
        launch_settings.public_key = public_key;

        let outcome = run_with_runtime(&launch_settings, 100, Vec::new(), &runtime).unwrap();
        assert!(matches!(
            outcome,
            LaunchOutcome::UpdatedAndStarted { ref version, .. } if version == "1.2.4"
        ));
        assert_eq!(fs::read(app.join("ostrog.exe")).unwrap(), b"new");
        assert!(!update_dir.join("backup").exists());
        assert!(!update_dir.join("staging/package-1-1").exists());
        assert_eq!(fs::read(data.join("world.save")).unwrap(), b"player data");
    }

    #[test]
    fn progress_sink_records_exact_stage_sequence_for_update_path() {
        let tree = TempTree::new("progress-update");
        let app = tree.path().join("app");
        let data = tree.path().join("data");
        fs::create_dir_all(&app).unwrap();
        fs::create_dir_all(&data).unwrap();
        fs::write(app.join("ostrog.exe"), b"old").unwrap();

        let (manifest, archive, public_key) = signed_archive();
        let total = archive.len() as u64;
        let mid = total / 2;
        let runtime = TestRuntime::with_manifest(manifest, archive);
        let mut launch_settings = settings();
        launch_settings.application_dir = app.clone();
        launch_settings.user_data_dir = data;
        launch_settings.public_key = public_key;

        let mut sink = RecordingSink::default();
        let outcome =
            run_with_runtime_and_progress(&launch_settings, 100, Vec::new(), &runtime, &mut sink)
                .unwrap();
        assert!(matches!(
            outcome,
            LaunchOutcome::UpdatedAndStarted { ref version, .. } if version == "1.2.4"
        ));
        assert_eq!(
            sink.events,
            vec![
                ProgressEvent {
                    stage: LauncherStage::Starting,
                    bytes: 0,
                    total: None,
                },
                ProgressEvent {
                    stage: LauncherStage::Checking,
                    bytes: 0,
                    total: None,
                },
                ProgressEvent {
                    stage: LauncherStage::Downloading {
                        version: "1.2.4".into(),
                    },
                    bytes: 0,
                    total: None,
                },
                ProgressEvent {
                    stage: LauncherStage::Downloading {
                        version: "1.2.4".into(),
                    },
                    bytes: mid,
                    total: Some(total),
                },
                ProgressEvent {
                    stage: LauncherStage::Downloading {
                        version: "1.2.4".into(),
                    },
                    bytes: total,
                    total: Some(total),
                },
                ProgressEvent {
                    stage: LauncherStage::Verifying,
                    bytes: 0,
                    total: None,
                },
                ProgressEvent {
                    stage: LauncherStage::Installing,
                    bytes: 0,
                    total: None,
                },
                ProgressEvent {
                    stage: LauncherStage::Launching,
                    bytes: 0,
                    total: None,
                },
            ]
        );
    }

    #[test]
    fn progress_sink_records_exact_stage_sequence_for_first_install_path() {
        let tree = TempTree::new("progress-install");
        let app = tree.path().join("app");
        let data = tree.path().join("data");
        // Empty app/ folder simulates first-install bootstrap.
        fs::create_dir_all(&data).unwrap();

        let (manifest, archive, public_key) = signed_archive();
        let total = archive.len() as u64;
        let mid = total / 2;
        let runtime = TestRuntime::with_manifest(manifest, archive);
        let mut launch_settings = settings();
        // Even if current_version matches the manifest, a missing app executable triggers install.
        launch_settings.current_version = "1.2.4".into();
        launch_settings.application_dir = app.clone();
        launch_settings.user_data_dir = data;
        launch_settings.public_key = public_key;

        let mut sink = RecordingSink::default();
        let outcome =
            run_with_runtime_and_progress(&launch_settings, 100, Vec::new(), &runtime, &mut sink)
                .unwrap();
        assert!(matches!(
            outcome,
            LaunchOutcome::UpdatedAndStarted { ref version, .. } if version == "1.2.4"
        ));
        assert_eq!(fs::read(app.join("ostrog.exe")).unwrap(), b"new");
        assert_eq!(
            sink.events,
            vec![
                ProgressEvent {
                    stage: LauncherStage::Starting,
                    bytes: 0,
                    total: None,
                },
                ProgressEvent {
                    stage: LauncherStage::Checking,
                    bytes: 0,
                    total: None,
                },
                ProgressEvent {
                    stage: LauncherStage::Downloading {
                        version: "1.2.4".into(),
                    },
                    bytes: 0,
                    total: None,
                },
                ProgressEvent {
                    stage: LauncherStage::Downloading {
                        version: "1.2.4".into(),
                    },
                    bytes: mid,
                    total: Some(total),
                },
                ProgressEvent {
                    stage: LauncherStage::Downloading {
                        version: "1.2.4".into(),
                    },
                    bytes: total,
                    total: Some(total),
                },
                ProgressEvent {
                    stage: LauncherStage::Verifying,
                    bytes: 0,
                    total: None,
                },
                ProgressEvent {
                    stage: LauncherStage::Installing,
                    bytes: 0,
                    total: None,
                },
                ProgressEvent {
                    stage: LauncherStage::Launching,
                    bytes: 0,
                    total: None,
                },
            ]
        );
    }

    #[test]
    fn progress_sink_records_exact_stage_sequence_for_offline_path() {
        let tree = TempTree::new("progress-offline");
        let app = tree.path().join("app");
        let data = tree.path().join("data");
        fs::create_dir_all(&app).unwrap();
        fs::create_dir_all(&data).unwrap();
        fs::write(app.join("ostrog.exe"), b"old").unwrap();

        let (manifest, archive, public_key) = signed_archive();
        let mut runtime = TestRuntime::with_manifest(manifest, archive);
        runtime.manifest_error = Some("server unreachable".into());
        let mut launch_settings = settings();
        launch_settings.application_dir = app;
        launch_settings.user_data_dir = data;
        launch_settings.public_key = public_key;

        let mut sink = RecordingSink::default();
        let outcome =
            run_with_runtime_and_progress(&launch_settings, 100, Vec::new(), &runtime, &mut sink)
                .unwrap();
        assert!(matches!(
            outcome,
            LaunchOutcome::StartedCurrent {
                reason: LaunchReason::Offline,
                ..
            }
        ));
        assert_eq!(
            sink.events,
            vec![
                ProgressEvent {
                    stage: LauncherStage::Starting,
                    bytes: 0,
                    total: None,
                },
                ProgressEvent {
                    stage: LauncherStage::Checking,
                    bytes: 0,
                    total: None,
                },
                ProgressEvent {
                    stage: LauncherStage::Offline,
                    bytes: 0,
                    total: None,
                },
                ProgressEvent {
                    stage: LauncherStage::Launching,
                    bytes: 0,
                    total: None,
                },
            ]
        );
    }

    #[test]
    fn progress_sink_records_exact_stage_sequence_for_failure_paths() {
        // 1. Update download fails with an installed build: falls back to Launching.
        let tree = TempTree::new("progress-failure-fallback");
        let app = tree.path().join("app");
        let data = tree.path().join("data");
        fs::create_dir_all(&app).unwrap();
        fs::create_dir_all(&data).unwrap();
        fs::write(app.join("ostrog.exe"), b"old").unwrap();

        let (manifest, archive, public_key) = signed_archive();
        let total = archive.len() as u64;
        let mid = total / 2;
        let mut runtime = TestRuntime::with_manifest(manifest.clone(), archive.clone());
        runtime.archive_error = Some("connection reset".into());
        let mut launch_settings = settings();
        launch_settings.application_dir = app;
        launch_settings.user_data_dir = data;
        launch_settings.public_key = public_key;

        let mut fallback_sink = RecordingSink::default();
        let outcome = run_with_runtime_and_progress(
            &launch_settings,
            100,
            Vec::new(),
            &runtime,
            &mut fallback_sink,
        )
        .unwrap();
        assert!(matches!(
            outcome,
            LaunchOutcome::StartedCurrent {
                reason: LaunchReason::UpdateFailed,
                ..
            }
        ));
        assert_eq!(
            fallback_sink.events,
            vec![
                ProgressEvent {
                    stage: LauncherStage::Starting,
                    bytes: 0,
                    total: None,
                },
                ProgressEvent {
                    stage: LauncherStage::Checking,
                    bytes: 0,
                    total: None,
                },
                ProgressEvent {
                    stage: LauncherStage::Downloading {
                        version: "1.2.4".into(),
                    },
                    bytes: 0,
                    total: None,
                },
                ProgressEvent {
                    stage: LauncherStage::Downloading {
                        version: "1.2.4".into(),
                    },
                    bytes: mid,
                    total: Some(total),
                },
                ProgressEvent {
                    stage: LauncherStage::Launching,
                    bytes: 0,
                    total: None,
                },
            ]
        );

        // 2. First-install download fails with no installed build: ends in Failure stage.
        let empty_tree = TempTree::new("progress-failure-fatal");
        let empty_app = empty_tree.path().join("app");
        let empty_data = empty_tree.path().join("data");
        fs::create_dir_all(&empty_data).unwrap();
        let mut fatal_settings = settings();
        fatal_settings.application_dir = empty_app;
        fatal_settings.user_data_dir = empty_data;
        fatal_settings.public_key = public_key;

        let mut fatal_sink = RecordingSink::default();
        let err = run_with_runtime_and_progress(
            &fatal_settings,
            100,
            Vec::new(),
            &runtime,
            &mut fatal_sink,
        )
        .unwrap_err();
        assert!(matches!(
            err,
            LauncherError::NoInstalledBuild {
                reason: LaunchReason::UpdateFailed
            }
        ));
        assert_eq!(fatal_sink.events.len(), 5);
        assert_eq!(fatal_sink.events[0].stage, LauncherStage::Starting);
        assert_eq!(fatal_sink.events[1].stage, LauncherStage::Checking);
        assert_eq!(
            fatal_sink.events[2].stage,
            LauncherStage::Downloading {
                version: "1.2.4".into()
            }
        );
        assert_eq!(
            fatal_sink.events[3],
            ProgressEvent {
                stage: LauncherStage::Downloading {
                    version: "1.2.4".into(),
                },
                bytes: mid,
                total: Some(total),
            }
        );
        assert!(matches!(
            fatal_sink.events[4].stage,
            LauncherStage::Failure { ref message } if message.contains("could not download update 1.2.4")
        ));
    }

    #[test]
    fn config_rejects_absolute_game_executables() {
        let text = r#"
manifest_url = "https://releases.example/manifest.json"
public_key = "0707070707070707070707070707070707070707070707070707070707070707"
current_version = "1.2.3"
channel = "stable"
timeout_seconds = 10
check_enabled = true
deferred_until = 0
application_dir = "app"
user_data_dir = "data"
executable = "C:/Windows/system32/cmd.exe"
"#;
        assert!(matches!(
            LauncherSettings::from_toml(text),
            Err(ConfigError::ExecutableMustBeRelative(_))
        ));
    }

    #[test]
    fn config_rejects_windows_absolute_paths_even_when_checked_on_linux() {
        let text = r#"
manifest_url = "https://releases.example/manifest.json"
public_key = "0707070707070707070707070707070707070707070707070707070707070707"
current_version = "1.2.3"
channel = "stable"
timeout_seconds = 10
check_enabled = true
deferred_until = 0
application_dir = "app"
user_data_dir = "data"
executable = "C:\\Games\\ostrog.exe"
"#;
        assert!(matches!(
            LauncherSettings::from_toml(text),
            Err(ConfigError::ExecutableMustBeRelative(_))
        ));
    }

    #[test]
    fn first_install_bootstrap_from_empty_folder_installs_app_and_persists_version() {
        let tree = TempTree::new("bootstrap-empty");
        let missing_config = tree.path().join("launcher.toml");
        assert!(!missing_config.exists());

        // 1. Load embedded default settings when launcher.toml does not exist.
        let mut settings = LauncherSettings::load_or_default(&missing_config).unwrap();
        assert_eq!(settings.manifest_url, DEFAULT_MANIFEST_URL);
        assert_eq!(hex::encode(settings.public_key), DEFAULT_PUBLIC_KEY_HEX);
        assert_eq!(settings.current_version, "0.0.0");
        assert_eq!(settings.application_dir, tree.path().join("app"));
        assert_eq!(settings.user_data_dir, tree.path().join("data"));
        assert!(!settings.has_installed_build());

        // 2. Run first-install bootstrap from the completely empty folder.
        let (manifest, archive, public_key) = signed_archive();
        settings.public_key = public_key;
        let runtime = TestRuntime::with_manifest(manifest, archive);
        let outcome = run_with_runtime(&settings, 100, Vec::new(), &runtime).unwrap();
        assert!(matches!(
            outcome,
            LaunchOutcome::UpdatedAndStarted { ref version, .. } if version == "1.2.4"
        ));
        assert!(settings.has_installed_build());
        assert_eq!(
            fs::read(tree.path().join("app/ostrog.exe")).unwrap(),
            b"new"
        );
        assert_eq!(
            fs::read_to_string(tree.path().join("app/VERSION"))
                .unwrap()
                .trim(),
            "1.2.4"
        );
        assert!(tree.path().join("data").is_dir());
        assert_eq!(settings.effective_current_version(), "1.2.4");

        // 3. A second launch with the same manifest sees the installed 1.2.4 build and starts it
        // directly as CurrentVersion without re-downloading.
        let second_outcome = run_with_runtime(&settings, 100, Vec::new(), &runtime).unwrap();
        assert!(matches!(
            second_outcome,
            LaunchOutcome::StartedCurrent {
                reason: LaunchReason::CurrentVersion,
                ..
            }
        ));
    }

    #[test]
    fn stage_format_status_matches_wp02_table_and_writes_launcher_log() {
        let mb = 1_048_576u64;
        assert_eq!(
            LauncherStage::Starting.format_status(true, 0, None, 0.0),
            "Starting Ostrog..."
        );
        assert_eq!(
            LauncherStage::Checking.format_status(true, 0, None, 0.0),
            "Checking for a newer version..."
        );
        assert_eq!(
            LauncherStage::Offline.format_status(true, 0, None, 0.0),
            "Can't reach the update server. Starting your installed version."
        );
        assert_eq!(
            LauncherStage::Offline.format_status(false, 0, None, 0.0),
            "Can't reach the server. Check your internet and try again."
        );
        assert_eq!(
            LauncherStage::Downloading {
                version: "0.4.1".into()
            }
            .format_status(true, 46 * mb, Some(104 * mb), 31.7),
            "Downloading Ostrog 0.4.1: 46 of 104 MB (44%), about 40 s left"
        );
        assert_eq!(
            LauncherStage::Verifying.format_status(true, 0, None, 0.0),
            "Checking the download is genuine..."
        );
        assert_eq!(
            LauncherStage::Installing.format_status(true, 0, None, 0.0),
            "Installing... this takes a moment"
        );
        assert_eq!(
            LauncherStage::Launching.format_status(true, 0, None, 0.0),
            "Starting the game. A window will open shortly."
        );
        assert_eq!(
            LauncherStage::Failure {
                message: "disk is full".into()
            }
            .format_status(true, 0, None, 0.0),
            "Something went wrong: disk is full. Your saved games are safe. Details: data\\logs\\launcher.log"
        );

        let tree = TempTree::new("launcher-log");
        let data = tree.path().join("data");
        let log_path = append_launcher_log(&data, "stage=starting").unwrap();
        assert_eq!(log_path, data.join("logs/launcher.log"));
        let log_text = fs::read_to_string(&log_path).unwrap();
        assert!(log_text.contains("stage=starting"));
    }
}
