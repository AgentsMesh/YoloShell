use super::constants::*;
use uuid::Uuid;
use yolo_hub_client_access_protocol::{
    ACCESS_SCHEMA_VERSION, AccessConfiguration, ClientGrant, ClientIdentity,
    ED25519_PUBLIC_KEY_BYTES, FIRST_ACCESS_REVISION, OriginAccess,
};
use yolo_hub_identity::HubId;
use yolo_hub_routing::{Right, Rights};

pub fn hub() -> HubId {
    HubId::from_uuid(Uuid::from_u128(HUB_VALUE)).unwrap()
}

pub fn other_hub() -> HubId {
    HubId::from_uuid(Uuid::from_u128(OTHER_HUB_VALUE)).unwrap()
}

pub fn rights(right: Right) -> Rights {
    [right].into_iter().collect()
}

pub fn configuration() -> AccessConfiguration {
    AccessConfiguration {
        schema_version: ACCESS_SCHEMA_VERSION,
        entry_hub: hub(),
        revision: FIRST_ACCESS_REVISION,
        clients: vec![ClientGrant {
            identity: ClientIdentity {
                client_id: Uuid::from_u128(CLIENT_VALUE),
                public_key: [CLIENT_KEY_BYTE; ED25519_PUBLIC_KEY_BYTES],
            },
            origins: vec![OriginAccess {
                origin: hub(),
                rights: rights(Right::Observe),
            }],
            expires_at_ms: EXPIRES_AT_MS,
        }],
    }
}

pub fn next(current: &AccessConfiguration) -> AccessConfiguration {
    let mut next = current.clone();
    next.revision += REVISION_STEP;
    next
}
