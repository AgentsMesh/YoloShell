mod constants;
mod snapshot_wire;
mod state_wire;
mod validation;
use anyhow::{Result, ensure};
pub use constants::*;
use serde::{Deserialize, Serialize};
use snapshot_wire::SnapshotWire;
use state_wire::StateWire;
use std::path::PathBuf;
use uuid::Uuid;
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_federation_settings::{Configuration, validate_socket};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigurationChange {
    #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
    pub request_id: Uuid,
    #[serde(deserialize_with = "validation::revision")]
    pub expected_revision: u64,
    pub configuration: Configuration,
}
impl ConfigurationChange {
    pub fn validate(&self, local: PeerIdentity) -> Result<()> {
        ensure!(
            !self.request_id.is_nil() && self.expected_revision >= FIRST_CONFIGURATION_REVISION,
            INVALID_CONFIGURATION_CHANGE
        );
        self.configuration.validate(local)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "StateWire", into = "StateWire")]
pub struct ConfigurationState {
    pub schema_version: u16,
    pub identity: PeerIdentity,
    pub revision: u64,
    pub last_change: Uuid,
    pub configuration: Configuration,
}
impl ConfigurationState {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.schema_version == CONFIGURATION_SCHEMA
                && self.revision >= FIRST_CONFIGURATION_REVISION
                && !self.last_change.is_nil(),
            INVALID_CONFIGURATION_STATE
        );
        self.configuration.validate(self.identity)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "SnapshotWire", into = "SnapshotWire")]
pub struct ConfigurationSnapshot {
    pub state: ConfigurationState,
    pub applied: bool,
    /// The published peer socket path when this generation is applied.
    pub peer_socket: Option<PathBuf>,
}
impl ConfigurationSnapshot {
    pub fn validate(&self) -> Result<()> {
        self.state.validate()?;
        match (self.applied, &self.peer_socket) {
            (false, None) => Ok(()),
            (true, Some(path)) => validate_socket(path),
            _ => anyhow::bail!(INVALID_CONFIGURATION_STATE),
        }
    }
}

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;
