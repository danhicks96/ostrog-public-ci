//! Build signed, public release manifests without putting the signing key in the client.
//!
//! The manifest format and authenticated payload are shared with `hf-update`. This crate is a
//! release-time tool only: its private key input is caller-supplied and never becomes part of a
//! package or launcher configuration.

use ed25519_dalek::{Signer, SigningKey};
use hf_update::{
    manifest_signing_payload, Architecture, Channel, Platform, ReleaseManifest, MANIFEST_SCHEMA,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{fmt, fs, path::Path};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SignedManifest {
    schema: u8,
    version: String,
    channel: &'static str,
    platform: &'static str,
    architecture: &'static str,
    archive_url: String,
    archive_sha256: String,
    signature: String,
}

impl SignedManifest {
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self).map(|json| format!("{json}\n"))
    }

    /// Parses the generated JSON through the client-side verifier before it is published.
    pub fn verify_with(
        &self,
        public_key: &[u8; 32],
        archive: &[u8],
    ) -> Result<(), hf_update::VerificationError> {
        let json = self.to_json().expect("manifest fields are serializable");
        let parsed = ReleaseManifest::parse(&json).expect("generated manifest must parse");
        parsed.verify_windows_x64(
            public_key,
            archive,
            &self.version,
            self.channel
                .parse()
                .expect("generated channel is one of the supported channels"),
        )
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum BuildError {
    InvalidVersion(String),
    InvalidArchiveUrl(String),
    InvalidArchivePath,
    InvalidSigningKeyLength(usize),
    InvalidGeneratedManifest(String),
    Io(String),
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for BuildError {}

/// Builds a signed Windows x64 manifest for the exact archive bytes to be published.
pub fn build_windows_manifest(
    version: &str,
    channel: Channel,
    archive_url: &str,
    archive: &[u8],
    signing_key: &[u8; 32],
) -> Result<SignedManifest, BuildError> {
    if !valid_version(version) {
        return Err(BuildError::InvalidVersion(version.to_owned()));
    }
    if !valid_https_url(archive_url) {
        return Err(BuildError::InvalidArchiveUrl(archive_url.to_owned()));
    }

    let archive_sha256 = hex::encode(Sha256::digest(archive));
    let payload = manifest_signing_payload(
        version,
        channel,
        Platform::Windows,
        Architecture::X86_64,
        archive_url,
        &archive_sha256,
    );
    let signature = SigningKey::from_bytes(signing_key).sign(&payload);
    let manifest = SignedManifest {
        schema: MANIFEST_SCHEMA,
        version: version.to_owned(),
        channel: channel.as_str(),
        platform: Platform::Windows.as_str(),
        architecture: Architecture::X86_64.as_str(),
        archive_url: archive_url.to_owned(),
        archive_sha256,
        signature: hex::encode(signature.to_bytes()),
    };

    // Parse before returning so a release job cannot publish bytes the launcher would reject.
    ReleaseManifest::parse(
        &manifest
            .to_json()
            .map_err(|e| BuildError::InvalidGeneratedManifest(e.to_string()))?,
    )
    .map_err(|e| BuildError::InvalidGeneratedManifest(e.to_string()))?;
    Ok(manifest)
}

pub fn read_signing_key(path: impl AsRef<Path>) -> Result<[u8; 32], BuildError> {
    read_key(path)
}

/// Reads the public half of the release key from an external raw or hex file.
pub fn read_public_key(path: impl AsRef<Path>) -> Result<[u8; 32], BuildError> {
    read_key(path)
}

fn read_key(path: impl AsRef<Path>) -> Result<[u8; 32], BuildError> {
    let bytes = fs::read(path).map_err(|e| BuildError::Io(e.to_string()))?;
    if bytes.len() == 32 {
        let length = bytes.len();
        return bytes
            .try_into()
            .map_err(|_| BuildError::InvalidSigningKeyLength(length));
    }
    let text = std::str::from_utf8(&bytes)
        .map(str::trim)
        .map_err(|_| BuildError::InvalidSigningKeyLength(bytes.len()))?;
    let decoded =
        hex::decode(text).map_err(|_| BuildError::InvalidSigningKeyLength(bytes.len()))?;
    let length = decoded.len();
    decoded
        .try_into()
        .map_err(|_| BuildError::InvalidSigningKeyLength(length))
}

pub fn write_manifest(path: impl AsRef<Path>, manifest: &SignedManifest) -> Result<(), BuildError> {
    let path = path.as_ref();
    let json = manifest
        .to_json()
        .map_err(|e| BuildError::InvalidGeneratedManifest(e.to_string()))?;
    fs::write(path, json).map_err(|e| BuildError::Io(e.to_string()))
}

fn valid_version(version: &str) -> bool {
    let mut parts = version.split('.');
    (0..3).all(|_| {
        parts.next().is_some_and(|part| {
            !part.is_empty()
                && (part == "0" || !part.starts_with('0'))
                && part.bytes().all(|byte| byte.is_ascii_digit())
        })
    }) && parts.next().is_none()
}

fn valid_https_url(url: &str) -> bool {
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

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::VerifyingKey;
    use std::io::Write;

    const KEY: [u8; 32] = [7; 32];
    const URL: &str =
        "https://github.com/example-user/example-repository/releases/download/c161/ostrog-windows-x64.zip";

    #[test]
    fn generated_manifest_verifies_against_the_client_contract() {
        let manifest =
            build_windows_manifest("1.2.3", Channel::Stable, URL, b"archive", &KEY).unwrap();
        let json = manifest.to_json().unwrap();
        let parsed = ReleaseManifest::parse(&json).unwrap();
        parsed
            .verify_windows_x64(
                &VerifyingKey::from(&SigningKey::from_bytes(&KEY)).to_bytes(),
                b"archive",
                "1.2.3",
                Channel::Stable,
            )
            .unwrap();
        assert_eq!(manifest.platform, "windows");
        assert_eq!(manifest.architecture, "x86_64");
    }

    #[test]
    fn archive_bytes_and_url_are_authenticated_and_deterministic() {
        let first = build_windows_manifest("1.2.3", Channel::Stable, URL, b"one", &KEY)
            .unwrap()
            .to_json()
            .unwrap();
        let second = build_windows_manifest("1.2.3", Channel::Stable, URL, b"one", &KEY)
            .unwrap()
            .to_json()
            .unwrap();
        let changed = build_windows_manifest("1.2.3", Channel::Stable, URL, b"two", &KEY)
            .unwrap()
            .to_json()
            .unwrap();
        assert_eq!(first, second);
        assert_ne!(first, changed);
        assert!(first.contains("\"archive_url\":"));
        assert!(first.contains("\"archive_sha256\":"));
    }

    #[test]
    fn invalid_release_inputs_are_rejected_before_signing() {
        assert_eq!(
            build_windows_manifest("01.2.3", Channel::Stable, URL, b"archive", &KEY),
            Err(BuildError::InvalidVersion("01.2.3".into()))
        );
        assert!(matches!(
            build_windows_manifest(
                "1.2.3",
                Channel::Stable,
                "http://example.com/a.zip",
                b"archive",
                &KEY
            ),
            Err(BuildError::InvalidArchiveUrl(_))
        ));
    }

    #[test]
    fn key_reader_accepts_raw_and_hex_files_without_shipping_key_material() {
        let directory = tempfile::tempdir().unwrap();
        let raw = directory.path().join("raw.key");
        fs::write(&raw, KEY).unwrap();
        assert_eq!(read_signing_key(&raw).unwrap(), KEY);

        let hex_path = directory.path().join("hex.key");
        let mut file = fs::File::create(&hex_path).unwrap();
        writeln!(file, "{}", hex::encode(KEY)).unwrap();
        assert_eq!(read_signing_key(&hex_path).unwrap(), KEY);
    }
}
