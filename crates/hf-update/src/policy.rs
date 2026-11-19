//! Deterministic startup policy around a future manifest transport.
//!
//! The transport remains outside this crate. Callers supply either a verified manifest or a
//! bounded failure result, and this policy decides whether the launcher offers an update or
//! starts the current build. No network, clock, filesystem, or credential access occurs here.

use crate::{validate_version, version_is_newer, Channel, ReleaseManifest};

const MAX_TIMEOUT_SECONDS: u32 = 30;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CheckConfig {
    pub enabled: bool,
    pub timeout_seconds: u32,
    pub deferred_until: Option<u64>,
}

impl CheckConfig {
    pub fn validate(self) -> Result<(), PolicyError> {
        if self.timeout_seconds == 0 || self.timeout_seconds > MAX_TIMEOUT_SECONDS {
            return Err(PolicyError::TimeoutOutOfRange {
                seconds: self.timeout_seconds,
                maximum: MAX_TIMEOUT_SECONDS,
            });
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FetchResult<'a> {
    Verified(&'a ReleaseManifest),
    Offline,
    TimedOut,
    Rejected,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FallbackReason {
    Offline,
    TimedOut,
    Rejected,
    WrongChannel,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CheckDecision {
    Disabled,
    Deferred,
    LaunchCurrent { reason: FallbackReason },
    CurrentVersion,
    Offer { version: String, channel: Channel },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PolicyError {
    TimeoutOutOfRange { seconds: u32, maximum: u32 },
    InvalidCurrentVersion(String),
}

/// Converts a verified fetch result into a non-blocking startup decision.
pub fn decide(
    config: CheckConfig,
    now: u64,
    current_version: &str,
    current_channel: Channel,
    fetched: FetchResult<'_>,
) -> Result<CheckDecision, PolicyError> {
    config.validate()?;
    if !config.enabled {
        return Ok(CheckDecision::Disabled);
    }
    if config.deferred_until.is_some_and(|until| now < until) {
        return Ok(CheckDecision::Deferred);
    }
    if validate_version(current_version).is_err() {
        return Err(PolicyError::InvalidCurrentVersion(
            current_version.to_owned(),
        ));
    }

    match fetched {
        FetchResult::Offline => Ok(CheckDecision::LaunchCurrent {
            reason: FallbackReason::Offline,
        }),
        FetchResult::TimedOut => Ok(CheckDecision::LaunchCurrent {
            reason: FallbackReason::TimedOut,
        }),
        FetchResult::Rejected => Ok(CheckDecision::LaunchCurrent {
            reason: FallbackReason::Rejected,
        }),
        FetchResult::Verified(manifest) => {
            if manifest.channel != current_channel {
                return Ok(CheckDecision::LaunchCurrent {
                    reason: FallbackReason::WrongChannel,
                });
            }
            if version_is_newer(&manifest.version, current_version) {
                Ok(CheckDecision::Offer {
                    version: manifest.version.clone(),
                    channel: manifest.channel,
                })
            } else {
                Ok(CheckDecision::CurrentVersion)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Architecture, Platform};
    use ed25519_dalek::{Signer, SigningKey};
    use serde_json::json;
    use sha2::{Digest, Sha256};

    fn manifest(version: &str, channel: &str) -> ReleaseManifest {
        let key = SigningKey::from_bytes(&[9; 32]);
        let archive_hash = hex::encode(Sha256::digest(b"archive"));
        let channel_value: Channel = channel.parse().unwrap();
        let archive_url = "https://releases.example/ostrog-windows-x64.zip";
        let payload = format!(
            "schema=1\nversion={version}\nchannel={channel}\nplatform=windows\narchitecture=x86_64\narchive_url={archive_url}\narchive_sha256={archive_hash}\n"
        );
        let json = json!({
            "schema": 1,
            "version": version,
            "channel": channel,
            "platform": "windows",
            "architecture": "x86_64",
            "archive_url": archive_url,
            "archive_sha256": archive_hash,
            "signature": hex::encode(key.sign(payload.as_bytes()).to_bytes()),
        });
        let parsed = ReleaseManifest::parse(&json.to_string()).unwrap();
        assert_eq!(parsed.platform, Platform::Windows);
        assert_eq!(parsed.architecture, Architecture::X86_64);
        assert_eq!(parsed.channel, channel_value);
        parsed
    }

    fn config() -> CheckConfig {
        CheckConfig {
            enabled: true,
            timeout_seconds: 10,
            deferred_until: None,
        }
    }

    #[test]
    fn offline_and_timeout_launch_the_current_version() {
        assert_eq!(
            decide(
                config(),
                100,
                "1.2.3",
                Channel::Stable,
                FetchResult::Offline
            )
            .unwrap(),
            CheckDecision::LaunchCurrent {
                reason: FallbackReason::Offline
            }
        );
        assert_eq!(
            decide(
                config(),
                100,
                "1.2.3",
                Channel::Stable,
                FetchResult::TimedOut
            )
            .unwrap(),
            CheckDecision::LaunchCurrent {
                reason: FallbackReason::TimedOut
            }
        );
    }

    #[test]
    fn disabled_and_deferred_checks_do_not_offer_an_update() {
        let disabled = CheckConfig {
            enabled: false,
            ..config()
        };
        assert_eq!(
            decide(
                disabled,
                100,
                "1.2.3",
                Channel::Stable,
                FetchResult::Offline
            )
            .unwrap(),
            CheckDecision::Disabled
        );

        let deferred = CheckConfig {
            deferred_until: Some(200),
            ..config()
        };
        assert_eq!(
            decide(
                deferred,
                100,
                "1.2.3",
                Channel::Stable,
                FetchResult::Verified(&manifest("9.9.9", "stable"))
            )
            .unwrap(),
            CheckDecision::Deferred
        );
    }

    #[test]
    fn only_a_newer_manifest_on_the_selected_channel_is_offered() {
        assert_eq!(
            decide(
                config(),
                100,
                "1.2.3",
                Channel::Stable,
                FetchResult::Verified(&manifest("1.2.3", "stable"))
            )
            .unwrap(),
            CheckDecision::CurrentVersion
        );
        assert_eq!(
            decide(
                config(),
                100,
                "1.2.3",
                Channel::Stable,
                FetchResult::Verified(&manifest("1.3.0", "stable"))
            )
            .unwrap(),
            CheckDecision::Offer {
                version: "1.3.0".into(),
                channel: Channel::Stable
            }
        );
        assert_eq!(
            decide(
                config(),
                100,
                "1.2.3",
                Channel::Stable,
                FetchResult::Verified(&manifest("9.9.9", "beta"))
            )
            .unwrap(),
            CheckDecision::LaunchCurrent {
                reason: FallbackReason::WrongChannel
            }
        );
    }

    #[test]
    fn timeout_is_bounded_and_current_version_is_checked() {
        let invalid_timeout = CheckConfig {
            timeout_seconds: 31,
            ..config()
        };
        assert_eq!(
            decide(
                invalid_timeout,
                100,
                "1.2.3",
                Channel::Stable,
                FetchResult::Offline
            ),
            Err(PolicyError::TimeoutOutOfRange {
                seconds: 31,
                maximum: 30
            })
        );
        assert_eq!(
            decide(config(), 100, "1.2", Channel::Stable, FetchResult::Offline),
            Err(PolicyError::InvalidCurrentVersion("1.2".into()))
        );
    }
}
