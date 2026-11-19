use crate::{BrokerHubConnectionBinding, BrokerHubError};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use yolo_hub_delegation::PeerIdentity;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrokerHubOpenRequest {
    pub binding: BrokerHubConnectionBinding,
    pub endpoint: SocketAddr,
    pub peer: PeerIdentity,
}

impl BrokerHubOpenRequest {
    pub fn validate(&self) -> Result<(), BrokerHubError> {
        self.binding.validate()?;
        if !self.endpoint.ip().is_loopback()
            || self.endpoint.port() == u16::default()
            || matches!(self.endpoint, SocketAddr::V6(value) if value.scope_id() != 0 || value.flowinfo() != 0)
        {
            return Err(BrokerHubError::Endpoint);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrokerHubOpened {
    pub request: BrokerHubOpenRequest,
    pub channel_id: u64,
}

impl BrokerHubOpened {
    pub fn validate(&self, expected: &BrokerHubOpenRequest) -> Result<(), BrokerHubError> {
        self.request.validate()?;
        if &self.request != expected {
            return Err(BrokerHubError::ChannelMismatch);
        }
        Ok(())
    }
}
