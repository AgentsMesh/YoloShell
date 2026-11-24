use crate::{EntryProfile, ProfileError};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;
use yolo_hub_client_access_protocol::ED25519_PUBLIC_KEY_BYTES;
use yolo_hub_client_transport::HubPin;
use yolo_hub_identity::HubId;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Wire {
    schema_version: u16,
    host: String,
    port: u16,
    username: String,
    outer_host_key: String,
    remote_socket: PathBuf,
    hub: Pin,
    #[serde(with = "yolo_crypto::serde_uuid")]
    client_id: Uuid,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Pin {
    hub_id: HubId,
    public_key: [u8; ED25519_PUBLIC_KEY_BYTES],
}
impl TryFrom<Wire> for EntryProfile {
    type Error = ProfileError;
    fn try_from(wire: Wire) -> Result<Self, Self::Error> {
        let profile = Self {
            schema_version: wire.schema_version,
            host: wire.host,
            port: wire.port,
            username: wire.username,
            outer_host_key: wire.outer_host_key,
            remote_socket: wire.remote_socket,
            hub: HubPin {
                hub_id: wire.hub.hub_id,
                public_key: wire.hub.public_key,
            },
            client_id: wire.client_id,
        };
        profile.validate()?;
        Ok(profile)
    }
}
impl From<EntryProfile> for Wire {
    fn from(profile: EntryProfile) -> Self {
        Self {
            schema_version: profile.schema_version,
            host: profile.host,
            port: profile.port,
            username: profile.username,
            outer_host_key: profile.outer_host_key,
            remote_socket: profile.remote_socket,
            hub: Pin {
                hub_id: profile.hub.hub_id,
                public_key: profile.hub.public_key,
            },
            client_id: profile.client_id,
        }
    }
}
