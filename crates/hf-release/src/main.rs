use clap::Parser;
use hf_release::{build_windows_manifest, read_public_key, read_signing_key, write_manifest};
use hf_update::{Channel, ReleaseManifest};
use std::{fs, path::PathBuf, str::FromStr};

#[derive(Debug, Parser)]
#[command(
    name = "hf-release-manifest",
    about = "Build a signed public Windows x64 release manifest"
)]
struct Args {
    #[arg(long)]
    version: Option<String>,
    #[arg(long, default_value = "stable")]
    channel: String,
    #[arg(long)]
    archive_url: Option<String>,
    #[arg(long)]
    archive: Option<PathBuf>,
    /// Path to an external 32-byte raw or 64-character hex Ed25519 signing key.
    #[arg(long)]
    signing_key: Option<PathBuf>,
    /// Path to the trusted 32-byte raw or 64-character hex Ed25519 public key.
    #[arg(long)]
    public_key: Option<PathBuf>,
    #[arg(long)]
    output: Option<PathBuf>,
    /// Verify an already-generated manifest before it is published.
    #[arg(long)]
    verify_manifest: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let channel = Channel::from_str(&args.channel)?;
    if let Some(manifest_path) = args.verify_manifest {
        let public_key_path = args
            .public_key
            .ok_or("--public-key is required with --verify-manifest")?;
        let archive_path = args
            .archive
            .ok_or("--archive is required with --verify-manifest")?;
        let json = fs::read_to_string(&manifest_path)?;
        let manifest = ReleaseManifest::parse(&json)?;
        let archive = fs::read(&archive_path)?;
        let public_key = read_public_key(public_key_path)?;
        let version = manifest.version.clone();
        manifest.verify_windows_x64(&public_key, &archive, &version, channel)?;
        println!("VERIFY OK {}", manifest_path.display());
        return Ok(());
    }

    let version = args.version.ok_or("--version is required when signing")?;
    let archive_url = args
        .archive_url
        .ok_or("--archive-url is required when signing")?;
    let archive_path = args.archive.ok_or("--archive is required when signing")?;
    let signing_key_path = args
        .signing_key
        .ok_or("--signing-key is required when signing")?;
    let output = args.output.ok_or("--output is required when signing")?;
    let archive = fs::read(archive_path)?;
    let key = read_signing_key(signing_key_path)?;
    let manifest = build_windows_manifest(&version, channel, &archive_url, &archive, &key)?;
    write_manifest(&output, &manifest)?;
    println!("SIGNED MANIFEST {}", output.display());
    Ok(())
}
