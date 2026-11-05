use crate::{BrokerHubConnectionBinding, BrokerHubError};
use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};
use yolo_hub_delegation::PeerIdentity;

const MAX_SOCKET_PATH_BYTES: usize = 100;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrokerHubOpenRequest {
    pub binding: BrokerHubConnectionBinding,
    pub socket: PathBuf,
    pub peer: PeerIdentity,
}

impl BrokerHubOpenRequest {
    pub fn validate(&self) -> Result<(), BrokerHubError> {
        self.binding.validate()?;
        validate_socket(&self.socket)
    }
}

fn validate_socket(path: &Path) -> Result<(), BrokerHubError> {
    let encoded = path.as_os_str().as_encoded_bytes();
    let raw = path.to_str().ok_or(BrokerHubError::Endpoint)?;
    if !path.is_absolute()
        || raw.split('/').any(|part| part == "." || part == "..")
        || encoded.is_empty()
        || encoded.len() > MAX_SOCKET_PATH_BYTES
        || encoded.iter().any(|byte| byte.is_ascii_control())
        || !path
            .components()
            .all(|component| matches!(component, Component::RootDir | Component::Normal(_)))
    {
        return Err(BrokerHubError::Endpoint);
    }
    Ok(())
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
