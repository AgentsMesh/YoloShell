use crate::{ConfigurationSnapshot, ConfigurationState};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SnapshotWire {
    state: ConfigurationState,
    applied: bool,
    peer_socket: Option<PathBuf>,
}

impl TryFrom<SnapshotWire> for ConfigurationSnapshot {
    type Error = anyhow::Error;
    fn try_from(wire: SnapshotWire) -> Result<Self, Self::Error> {
        let snapshot = Self {
            state: wire.state,
            applied: wire.applied,
            peer_socket: wire.peer_socket,
        };
        snapshot.validate()?;
        Ok(snapshot)
    }
}

impl From<ConfigurationSnapshot> for SnapshotWire {
    fn from(snapshot: ConfigurationSnapshot) -> Self {
        Self {
            state: snapshot.state,
            applied: snapshot.applied,
            peer_socket: snapshot.peer_socket,
        }
    }
}
