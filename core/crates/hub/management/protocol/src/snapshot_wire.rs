use crate::{ConfigurationSnapshot, ConfigurationState};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SnapshotWire {
    state: ConfigurationState,
    applied: bool,
    listen_address: Option<SocketAddr>,
}

impl TryFrom<SnapshotWire> for ConfigurationSnapshot {
    type Error = anyhow::Error;
    fn try_from(wire: SnapshotWire) -> Result<Self, Self::Error> {
        let snapshot = Self {
            state: wire.state,
            applied: wire.applied,
            listen_address: wire.listen_address,
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
            listen_address: snapshot.listen_address,
        }
    }
}
