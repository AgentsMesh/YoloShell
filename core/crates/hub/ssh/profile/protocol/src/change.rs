use crate::{SshProfileConfiguration, constants::*};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "ChangeWire", into = "ChangeWire")]
pub struct SshProfileChange {
    pub request_id: Uuid,
    pub expected_revision: u64,
    pub configuration: SshProfileConfiguration,
}
impl SshProfileChange {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            !self.request_id.is_nil() && self.expected_revision >= FIRST_SSH_PROFILE_REVISION,
            INVALID_CHANGE
        );
        self.configuration.validate()
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ChangeWire {
    request_id: Uuid,
    expected_revision: u64,
    configuration: SshProfileConfiguration,
}
impl TryFrom<ChangeWire> for SshProfileChange {
    type Error = anyhow::Error;
    fn try_from(value: ChangeWire) -> Result<Self> {
        let change = Self {
            request_id: value.request_id,
            expected_revision: value.expected_revision,
            configuration: value.configuration,
        };
        change.validate()?;
        Ok(change)
    }
}
impl From<SshProfileChange> for ChangeWire {
    fn from(value: SshProfileChange) -> Self {
        Self {
            request_id: value.request_id,
            expected_revision: value.expected_revision,
            configuration: value.configuration,
        }
    }
}
