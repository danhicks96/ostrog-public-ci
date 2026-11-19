#![cfg_attr(not(test), windows_subsystem = "windows")]

use clap::Parser;
use hf_launcher::{run, LauncherSettings};
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Parser)]
#[command(
    name = "ostrog-launcher",
    about = "Check, update and start Ostrog safely"
)]
struct Args {
    /// TOML config shipped beside the launcher or selected by the user.
    #[arg(long)]
    config: Option<PathBuf>,

    /// Run without opening the Win32 status window (for automated CLI scripts).
    #[arg(long)]
    headless: bool,

    /// Render a PNG screenshot of every WP-02 launcher stage into the given directory and exit.
    #[cfg(windows)]
    #[arg(long)]
    dump_stage_screenshots: Option<PathBuf>,

    /// Arguments passed unchanged to the game executable.
    #[arg(last = true)]
    game_args: Vec<OsString>,
}

fn default_config_path() -> PathBuf {
    let cwd_config = PathBuf::from("launcher.toml");
    if cwd_config.exists() {
        return cwd_config;
    }
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            return exe_dir.join("launcher.toml");
        }
    }
    Path::new("launcher.toml").to_path_buf()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let settings = match &args.config {
        Some(path) => LauncherSettings::load(path)?,
        None => LauncherSettings::load_or_default(default_config_path())?,
    };

    #[cfg(windows)]
    if let Some(dir) = &args.dump_stage_screenshots {
        hf_launcher::window::dump_stage_screenshots(&settings, dir)?;
        return Ok(());
    }

    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

    #[cfg(windows)]
    if !args.headless {
        return hf_launcher::window::run_windowed(settings, now, args.game_args)
            .map_err(Into::into);
    }

    let outcome = run(&settings, now, args.game_args)?;
    match outcome {
        hf_launcher::LaunchOutcome::StartedCurrent { reason, .. } => {
            eprintln!("starting installed Ostrog ({reason:?})");
        }
        hf_launcher::LaunchOutcome::UpdatedAndStarted { version, .. } => {
            eprintln!("updated Ostrog to {version} and started it");
        }
    }
    Ok(())
}
