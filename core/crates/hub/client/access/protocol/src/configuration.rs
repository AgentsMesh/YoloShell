use crate::{ACCESS_SCHEMA_VERSION, AccessError, ClientGrant, FIRST_ACCESS_REVISION, MAX_CLIENTS};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use yolo_hub_identity::HubId;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "ConfigurationWire")]
pub struct AccessConfiguration {
    pub schema_version: u16,
    pub entry_hub: HubId,
    pub revision: u64,
    pub clients: Vec<ClientGrant>,
}

impl AccessConfiguration {
    pub fn validate(&self) -> Result<(), AccessError> {
        if self.schema_version != ACCESS_SCHEMA_VERSION || self.revision < FIRST_ACCESS_REVISION {
            return Err(AccessError::Configuration);
        }
        if self.clients.len() > MAX_CLIENTS {
            return Err(AccessError::Capacity);
        }
        let mut identities = BTreeSet::new();
        let mut keys = BTreeSet::new();
        for grant in &self.clients {
            grant.validate()?;
            if !identities.insert(grant.identity.client_id)
                || !keys.insert(grant.identity.public_key)
            {
                return Err(AccessError::DuplicateClient);
            }
        }
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfigurationWire {
    schema_version: u16,
    entry_hub: HubId,
    revision: u64,
    clients: Vec<ClientGrant>,
}

impl TryFrom<ConfigurationWire> for AccessConfiguration {
    type Error = AccessError;

    fn try_from(wire: ConfigurationWire) -> Result<Self, Self::Error> {
        let configuration = Self {
            schema_version: wire.schema_version,
            entry_hub: wire.entry_hub,
            revision: wire.revision,
            clients: wire.clients,
        };
        configuration.validate()?;
        Ok(configuration)
    }
}
