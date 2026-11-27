use crate::{SshBrokerProfile, constants::*};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "ConfigurationWire", into = "ConfigurationWire")]
pub struct SshProfileConfiguration {
    pub schema_version: u16,
    pub profiles: Vec<SshBrokerProfile>,
}
impl Default for SshProfileConfiguration {
    fn default() -> Self {
        Self {
            schema_version: SSH_PROFILE_SCHEMA_VERSION,
            profiles: Vec::new(),
        }
    }
}
impl SshProfileConfiguration {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.schema_version == SSH_PROFILE_SCHEMA_VERSION
                && self.profiles.len() <= MAX_SSH_PROFILES,
            INVALID_CONFIGURATION
        );
        let mut ids = BTreeSet::new();
        let mut directories = BTreeSet::new();
        for profile in &self.profiles {
            profile.validate()?;
            ensure!(
                ids.insert(profile.profile.profile_id())
                    && directories.insert(&profile.broker_state),
                INVALID_CONFIGURATION
            );
        }
        Ok(())
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfigurationWire {
    schema_version: u16,
    profiles: Vec<SshBrokerProfile>,
}
impl TryFrom<ConfigurationWire> for SshProfileConfiguration {
    type Error = anyhow::Error;
    fn try_from(value: ConfigurationWire) -> Result<Self> {
        let configuration = Self {
            schema_version: value.schema_version,
            profiles: value.profiles,
        };
        configuration.validate()?;
        Ok(configuration)
    }
}
impl From<SshProfileConfiguration> for ConfigurationWire {
    fn from(value: SshProfileConfiguration) -> Self {
        Self {
            schema_version: value.schema_version,
            profiles: value.profiles,
        }
    }
}
