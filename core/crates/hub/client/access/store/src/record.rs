use crate::constants::*;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use yolo_hub_client_access_protocol::AccessConfiguration;
use yolo_hub_identity::HubId;
use yolo_paths::identity::DurableObjectIdentity;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Record {
    pub schema_version: u16,
    pub directory: DurableObjectIdentity,
    pub configuration: AccessConfiguration,
}

impl Record {
    pub fn validate(&self, directory: DurableObjectIdentity, entry_hub: HubId) -> Result<()> {
        ensure!(self.schema_version == PRIVATE_RECORD_SCHEMA, INVALID_SCHEMA);
        ensure!(
            self.directory == directory && self.configuration.entry_hub == entry_hub,
            WRONG_IDENTITY
        );
        self.configuration.validate()?;
        Ok(())
    }
}
