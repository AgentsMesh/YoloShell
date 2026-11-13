use crate::constants::*;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use yolo_ssh_profile_protocol::{PROFILE_REVISION_HEX_BYTES, SshProfileRef};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "ProfileWire", into = "ProfileWire")]
pub struct SshBrokerProfile {
    pub profile: SshProfileRef,
    pub broker_state: PathBuf,
    pub config_revision: String,
    pub launch: Option<crate::SshBrokerLaunch>,
}
impl SshBrokerProfile {
    pub fn validate(&self) -> Result<()> {
        validate_path(&self.broker_state)?;
        if let Some(launch) = &self.launch {
            launch.validate()?;
        }
        ensure!(
            self.config_revision.len() == PROFILE_REVISION_HEX_BYTES
                && self
                    .config_revision
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f')),
            INVALID_PROFILE
        );
        Ok(())
    }
}

pub(crate) fn validate_path(path: &std::path::Path) -> Result<()> {
    let path = path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!(INVALID_PROFILE))?;
    let relative = path
        .strip_prefix('/')
        .ok_or_else(|| anyhow::anyhow!(INVALID_PROFILE))?;
    ensure!(
        !relative.is_empty()
            && path.len() <= MAX_BROKER_STATE_PATH_BYTES
            && !path.chars().any(char::is_control)
            && relative
                .split('/')
                .all(|part| !part.is_empty() && part != "." && part != ".."),
        INVALID_PROFILE
    );
    Ok(())
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProfileWire {
    profile: SshProfileRef,
    broker_state: PathBuf,
    config_revision: String,
    #[serde(default)]
    launch: Option<crate::SshBrokerLaunch>,
}
impl TryFrom<ProfileWire> for SshBrokerProfile {
    type Error = anyhow::Error;
    fn try_from(value: ProfileWire) -> Result<Self> {
        let profile = Self {
            profile: value.profile,
            broker_state: value.broker_state,
            config_revision: value.config_revision,
            launch: value.launch,
        };
        profile.validate()?;
        Ok(profile)
    }
}
impl From<SshBrokerProfile> for ProfileWire {
    fn from(value: SshBrokerProfile) -> Self {
        Self {
            profile: value.profile,
            broker_state: value.broker_state,
            config_revision: value.config_revision,
            launch: value.launch,
        }
    }
}
