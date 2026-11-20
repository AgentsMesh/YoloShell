use crate::{
    SshProfileState,
    constants::{IDENTITY_CHANGED, RECORD_SCHEMA},
};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use yolo_hub_delegation::PeerIdentity;
use yolo_paths::identity::DurableObjectIdentity;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Record {
    pub schema_version: u16,
    pub directory: DurableObjectIdentity,
    pub state: SshProfileState,
}
impl Record {
    pub(crate) fn validate(
        &self,
        directory: DurableObjectIdentity,
        hub: PeerIdentity,
    ) -> Result<()> {
        ensure!(
            self.schema_version == RECORD_SCHEMA
                && self.directory == directory
                && self.state.hub == hub,
            IDENTITY_CHANGED
        );
        self.state.validate()
    }
}
