use crate::{ProfileError, constants::*, validation, wire::Wire};
use russh::keys::PublicKey;
use serde::{Deserialize, Serialize};
use std::{fmt, path::PathBuf};
use uuid::Uuid;
use yolo_hub_client_transport::HubPin;

/// Public connection configuration; credentials are supplied separately in memory.
#[derive(Clone, Serialize, Deserialize)]
#[serde(try_from = "Wire", into = "Wire")]
pub struct EntryProfile {
    pub schema_version: u16,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub outer_host_key: String,
    pub remote_socket: PathBuf,
    pub hub: HubPin,
    pub client_id: Uuid,
}

impl EntryProfile {
    pub fn decode(bytes: &[u8]) -> Result<Self, ProfileError> {
        if bytes.len() > MAX_PROFILE_BYTES {
            return Err(ProfileError::Size);
        }
        serde_json::from_slice(bytes).map_err(|_| ProfileError::Encoding)
    }
    pub fn validate(&self) -> Result<(), ProfileError> {
        if self.schema_version != ENTRY_PROFILE_SCHEMA {
            return Err(ProfileError::Schema);
        }
        if !validation::host(&self.host) {
            return Err(ProfileError::Host);
        }
        if self.port == u16::default() {
            return Err(ProfileError::Port);
        }
        if !validation::username(&self.username) {
            return Err(ProfileError::Username);
        }
        if !validation::socket(&self.remote_socket) {
            return Err(ProfileError::Socket);
        }
        if self.hub.public_key == EMPTY_KEY {
            return Err(ProfileError::HubPin);
        }
        if self.client_id.is_nil() {
            return Err(ProfileError::ClientId);
        }
        self.pinned_key()?;
        Ok(())
    }
    pub(crate) fn pinned_key(&self) -> Result<PublicKey, ProfileError> {
        let text = &self.outer_host_key;
        if text.is_empty() || text.len() > MAX_HOST_KEY_BYTES || text.chars().any(char::is_control)
        {
            return Err(ProfileError::HostKey);
        }
        PublicKey::from_openssh(text).map_err(|_| ProfileError::HostKey)
    }
}

impl fmt::Debug for EntryProfile {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EntryProfile")
            .field(FIELD_SCHEMA_VERSION, &self.schema_version)
            .field(FIELD_HUB_ID, &self.hub.hub_id)
            .field(FIELD_CLIENT_ID, &self.client_id)
            .finish_non_exhaustive()
    }
}
