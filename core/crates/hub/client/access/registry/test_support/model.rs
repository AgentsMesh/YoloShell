use yolo_hub_client_access_protocol::{
    ACCESS_SCHEMA_VERSION, AccessConfiguration, ClientGrant, ClientIdentity, FIRST_ACCESS_REVISION,
    OriginAccess,
};
use yolo_hub_identity::HubId;
use yolo_hub_routing::{Right, Rights};

#[path = "constants.rs"]
mod constants;
pub use constants::*;

pub fn configuration() -> AccessConfiguration {
    let entry_hub = HubId::new();
    AccessConfiguration {
        schema_version: ACCESS_SCHEMA_VERSION,
        entry_hub,
        revision: FIRST_ACCESS_REVISION,
        clients: vec![ClientGrant {
            identity: ClientIdentity {
                client_id: uuid::Uuid::new_v4(),
                public_key: CLIENT_KEY,
            },
            origins: vec![OriginAccess {
                origin: entry_hub,
                rights: rights(Right::TerminalRead),
            }],
            expires_at_ms: EXPIRES_AT_MS,
        }],
    }
}

pub fn next(configuration: &AccessConfiguration) -> AccessConfiguration {
    let mut next = configuration.clone();
    next.revision += REVISION_STEP;
    next
}

pub fn rights(right: Right) -> Rights {
    [right].into_iter().collect()
}
