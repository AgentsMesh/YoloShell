use crate::constants::{
    INVALID_PRIVATE_SCHEMA, LEGACY_PRIVATE_RECORD_SCHEMA, PRIVATE_RECORD_SCHEMA, WRONG_IDENTITY,
};
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
    /// 0.1.x wrote listener-binding evidence here. It is accepted on read and
    /// omitted on the next write; the peer socket now lives in Hub state.
    #[serde(default, rename = "listener_binding_initialized", skip_serializing)]
    pub(crate) _legacy_listener_binding: bool,
}

impl Record {
    pub fn validate(&self, directory: DurableObjectIdentity, identity: PeerIdentity) -> Result<()> {
        ensure!(
            self.schema_version == PRIVATE_RECORD_SCHEMA
                || self.schema_version == LEGACY_PRIVATE_RECORD_SCHEMA,
            INVALID_PRIVATE_SCHEMA
        );
        ensure!(
            self.directory == directory && self.state.identity == identity,
            WRONG_IDENTITY
        );
        self.state.validate()
    }
}
