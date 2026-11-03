use crate::constants::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use yolo_hub_identity::HubId;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadyDocument {
    pub schema_version: u16,
    pub entry_hub: HubId,
    pub socket_path: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectionDocument {
    pub schema_version: u16,
    pub entry_hub: HubId,
    pub socket_path: PathBuf,
    pub local_origin: HubId,
}

impl ConnectionDocument {
    pub fn local(hub: HubId, socket: PathBuf) -> Self {
        Self {
            schema_version: CONFIG_SCHEMA_VERSION,
            entry_hub: hub,
            socket_path: socket,
            local_origin: hub,
        }
    }
}
