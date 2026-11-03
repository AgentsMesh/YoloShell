use super::constants::*;
use uuid::Uuid;
use yolo_hub_client_access_protocol::*;
use yolo_hub_identity::HubId;
use yolo_hub_routing::{Right, Rights};

pub fn identity(index: usize) -> ClientIdentity {
    let client_id = Uuid::from_u128(FIRST_CLIENT_VALUE + u128::try_from(index).unwrap());
    let mut public_key = [KEY_FILL; ED25519_PUBLIC_KEY_BYTES];
    public_key[..client_id.as_bytes().len()].copy_from_slice(client_id.as_bytes());
    ClientIdentity {
        client_id,
        public_key,
    }
}

pub fn hub(index: usize) -> HubId {
    HubId::from_uuid(Uuid::from_u128(
        FIRST_HUB_VALUE + u128::try_from(index).unwrap(),
    ))
    .unwrap()
}

pub fn rights(right: Right) -> Rights {
    [right].into_iter().collect()
}

pub fn origin(index: usize, right: Right) -> OriginAccess {
    OriginAccess {
        origin: hub(index),
        rights: rights(right),
    }
}

pub fn grant(index: usize) -> ClientGrant {
    ClientGrant {
        identity: identity(index),
        origins: vec![origin(FIRST_INDEX, Right::Observe)],
        expires_at_ms: EXPIRES_AT_MS,
    }
}

pub fn configuration() -> AccessConfiguration {
    AccessConfiguration {
        schema_version: ACCESS_SCHEMA_VERSION,
        entry_hub: hub(FIRST_INDEX),
        revision: FIRST_ACCESS_REVISION,
        clients: vec![grant(FIRST_INDEX)],
    }
}
