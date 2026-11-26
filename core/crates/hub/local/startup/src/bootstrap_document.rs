use crate::{ConnectionDocument, constants::*};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use yolo_hub_state::HubState;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BootstrapDocument {
    pub schema_version: u16,
    pub state_path: PathBuf,
    pub host_home: PathBuf,
    pub connection: ConnectionDocument,
}

impl BootstrapDocument {
    pub fn new(state: &Path, home: &Path, connection: ConnectionDocument) -> Result<Self> {
        let state_path = state.canonicalize()?;
        let host_home = home.canonicalize()?;
        ensure!(
            HubState::inspect_identity(&state_path, &host_home)? == Some(connection.entry_hub)
                && HubState::socket_path_for(&state_path)? == connection.socket_path
                && connection.schema_version == CONFIG_SCHEMA_VERSION
                && connection.local_origin == connection.entry_hub,
            READY_INVALID
        );
        Ok(Self {
            schema_version: BOOTSTRAP_SCHEMA_VERSION,
            state_path,
            host_home,
            connection,
        })
    }
}
