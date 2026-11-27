#[path = "constants.rs"]
pub mod constants;
use constants::*;
use tokio::time::Instant;
use uuid::Uuid;
use yolo_hub_identity::{HubId, RuntimeEpoch, RuntimeTarget, TerminalAddress, TerminalId};
use yolo_hub_rpc_protocol::Command;
use yolo_hub_terminal_protocol::*;

pub fn id(value: u128) -> Uuid {
    Uuid::from_u128(value)
}
pub fn hub() -> HubId {
    HubId::from_uuid(id(HUB_ID)).unwrap()
}
pub fn other_hub() -> HubId {
    HubId::from_uuid(id(OTHER_HUB_ID)).unwrap()
}
pub fn deadline() -> Instant {
    Instant::now() + TEST_TIMEOUT
}
pub fn target() -> RuntimeTarget {
    RuntimeTarget {
        address: TerminalAddress {
            hub_id: hub(),
            terminal_id: TerminalId::from_uuid(id(SESSION_ID)).unwrap(),
        },
        runtime_epoch: RuntimeEpoch::from_uuid(id(RUNTIME_ID)).unwrap(),
    }
}
pub fn grant() -> LeaseGrant {
    LeaseGrant::new(LeaseGrantFields {
        target: target(),
        principal: WriterPrincipal {
            client_id: id(CLIENT_ID),
            controller_instance_id: id(CONTROLLER_ID),
        },
        generation: FIRST_LEASE_GENERATION,
        capability: CAPABILITY.to_owned().try_into().unwrap(),
        expires_at_ms: LEASE_EXPIRY,
        ttl_ms: LEASE_TTL,
        renew_after_ms: LEASE_RENEWAL,
    })
    .unwrap()
}
pub fn input() -> Command {
    Command::Input {
        request: InputRequest::new(InputFields {
            request_id: id(REQUEST_ID),
            grant: grant(),
            data: INPUT.to_vec(),
        })
        .unwrap(),
    }
}
pub fn list() -> Command {
    Command::List {
        include_archived: false,
        after: None,
    }
}
pub fn invalid() -> Command {
    Command::Resize {
        request: ResizeRequest {
            request_id: Uuid::nil(),
            grant: grant(),
            size: TerminalSize::default(),
        },
    }
}
