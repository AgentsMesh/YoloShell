use crate::constants::*;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use yolo_ssh_profile_protocol::PROFILE_REVISION_HEX_BYTES;

/// A public reference to a private local configuration source, never credentials.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "LaunchWire", into = "LaunchWire")]
pub struct SshBrokerLaunch {
    pub config_source: PathBuf,
    pub source_sha256: String,
}

impl SshBrokerLaunch {
    pub fn validate(&self) -> Result<()> {
        crate::profile::validate_path(&self.config_source)?;
        ensure!(
            self.source_sha256.len() == PROFILE_REVISION_HEX_BYTES
                && self
                    .source_sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f')),
            INVALID_PROFILE
        );
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LaunchWire {
    config_source: PathBuf,
    source_sha256: String,
}
impl TryFrom<LaunchWire> for SshBrokerLaunch {
    type Error = anyhow::Error;
    fn try_from(value: LaunchWire) -> Result<Self> {
        let launch = Self {
            config_source: value.config_source,
            source_sha256: value.source_sha256,
        };
        launch.validate()?;
        Ok(launch)
    }
}
impl From<SshBrokerLaunch> for LaunchWire {
    fn from(value: SshBrokerLaunch) -> Self {
        Self {
            config_source: value.config_source,
            source_sha256: value.source_sha256,
        }
    }
}
