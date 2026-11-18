use crate::{SshProfileConfiguration, constants::*};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use yolo_hub_delegation::PeerIdentity;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "StateWire", into = "StateWire")]
pub struct SshProfileState {
    pub hub: PeerIdentity,
    pub revision: u64,
    pub last_change: Uuid,
    pub configuration: SshProfileConfiguration,
}
impl SshProfileState {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.revision >= FIRST_SSH_PROFILE_REVISION && !self.last_change.is_nil(),
            INVALID_STATE
        );
        self.configuration.validate()
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StateWire {
    hub: PeerIdentity,
    revision: u64,
    last_change: Uuid,
    configuration: SshProfileConfiguration,
}
impl TryFrom<StateWire> for SshProfileState {
    type Error = anyhow::Error;
    fn try_from(value: StateWire) -> Result<Self> {
        let state = Self {
            hub: value.hub,
            revision: value.revision,
            last_change: value.last_change,
            configuration: value.configuration,
        };
        state.validate()?;
        Ok(state)
    }
}
impl From<SshProfileState> for StateWire {
    fn from(value: SshProfileState) -> Self {
        Self {
            hub: value.hub,
            revision: value.revision,
            last_change: value.last_change,
            configuration: value.configuration,
        }
    }
}
