use crate::constants::*;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_management_protocol::{CONFIGURATION_REVISION_STEP, ConfigurationState};
use yolo_paths::identity::DurableObjectIdentity;

pub fn persistent(address: Option<SocketAddr>) -> bool {
    address.is_some_and(|address| address.ip().is_loopback() && address.port() == EPHEMERAL_PORT)
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Entry {
    pub configuration: ConfigurationState,
    #[serde(deserialize_with = "required_option")]
    pub address: Option<SocketAddr>,
}

impl Entry {
    pub fn validate(&self, peer: PeerIdentity) -> Result<()> {
        self.configuration.validate()?;
        ensure!(self.configuration.identity == peer, INVALID);
        if let Some(address) = self.address {
            let requested = self.configuration.configuration.listen;
            ensure!(
                persistent(requested)
                    && requested.is_some_and(|requested| requested.ip() == address.ip())
                    && address.port() != EPHEMERAL_PORT,
                INVALID
            );
        }
        Ok(())
    }

    pub fn ready(&self) -> Result<()> {
        ensure!(
            !persistent(self.configuration.configuration.listen) || self.address.is_some(),
            INVALID
        );
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Record {
    pub schema: u16,
    pub root: DurableObjectIdentity,
    pub peer: PeerIdentity,
    pub current: Entry,
    #[serde(deserialize_with = "required_option")]
    pub pending: Option<Entry>,
}

fn required_option<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    deserializer: D,
) -> std::result::Result<Option<T>, D::Error> {
    Deserialize::deserialize(deserializer)
}

impl Record {
    pub fn validate(&self, root: DurableObjectIdentity, peer: PeerIdentity) -> Result<()> {
        ensure!(
            self.schema == SCHEMA && self.root == root && self.peer == peer,
            INVALID
        );
        self.current.validate(peer)?;
        if let Some(pending) = &self.pending {
            self.current.ready()?;
            pending.validate(peer)?;
            pending.ready()?;
            ensure!(
                self.current
                    .configuration
                    .revision
                    .checked_add(CONFIGURATION_REVISION_STEP)
                    == Some(pending.configuration.revision)
                    && self.current.configuration.last_change != pending.configuration.last_change,
                INVALID
            );
        }
        Ok(())
    }

    pub fn selected(&self, configuration: &ConfigurationState) -> Result<&Entry> {
        std::iter::once(&self.current)
            .chain(self.pending.iter())
            .find(|entry| entry.configuration == *configuration)
            .ok_or_else(|| anyhow::anyhow!(CONFLICT))
    }
}
