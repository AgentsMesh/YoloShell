use super::constants::*;
use uuid::Uuid;
use yolo_hub_client_access_protocol::{
    ACCESS_SCHEMA_VERSION, AccessConfiguration, ClientAccessChange, ClientGrant, ClientIdentity,
    ED25519_PUBLIC_KEY_BYTES, FIRST_ACCESS_REVISION, OriginAccess,
};
use yolo_hub_identity::HubId;
use yolo_hub_routing::Right;

pub fn client_key() -> [u8; ED25519_PUBLIC_KEY_BYTES] {
    [KEY_BYTE; ED25519_PUBLIC_KEY_BYTES]
}

pub fn configuration(entry_hub: HubId) -> AccessConfiguration {
    AccessConfiguration {
        schema_version: ACCESS_SCHEMA_VERSION,
        entry_hub,
        revision: FIRST_ACCESS_REVISION,
        clients: vec![ClientGrant {
            identity: ClientIdentity {
                client_id: Uuid::from_u128(CLIENT_VALUE),
                public_key: client_key(),
            },
            origins: vec![OriginAccess {
                origin: entry_hub,
                rights: [Right::Observe].into_iter().collect(),
            }],
            expires_at_ms: u64::MAX,
        }],
    }
}

pub fn change(current: &AccessConfiguration) -> ClientAccessChange {
    let mut next = current.clone();
    next.revision += REVISION_STEP;
    ClientAccessChange {
        request_id: Uuid::new_v4(),
        expected_revision: current.revision,
        configuration: next,
    }
}
