use crate::constants::{INVALID_PRIVATE_SCHEMA, PRIVATE_RECORD_SCHEMA, WRONG_IDENTITY};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_management_protocol::ConfigurationState;
use yolo_paths::identity::DurableObjectIdentity;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Record {
    pub schema_version: u16,
    pub directory: DurableObjectIdentity,
    pub state: ConfigurationState,
    pub listener_binding_initialized: bool,
}

impl Record {
    pub fn validate(&self, directory: DurableObjectIdentity, identity: PeerIdentity) -> Result<()> {
        ensure!(
            self.schema_version == PRIVATE_RECORD_SCHEMA,
            INVALID_PRIVATE_SCHEMA
        );
        ensure!(
            self.directory == directory && self.state.identity == identity,
            WRONG_IDENTITY
        );
        self.state.validate()
    }
}
