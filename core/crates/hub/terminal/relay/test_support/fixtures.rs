use super::constants::*;
use tokio::time::Instant;
use uuid::Uuid;
use yolo_hub_identity::{HubId, RuntimeEpoch, RuntimeTarget, TerminalAddress, TerminalId};
use yolo_hub_terminal_protocol::*;

pub fn hub() -> HubId {
    HubId::from_uuid(Uuid::from_u128(HUB_VALUE)).unwrap()
}
pub fn origin() -> HubId {
    HubId::from_uuid(Uuid::from_u128(ORIGIN_VALUE)).unwrap()
}
pub fn target() -> RuntimeTarget {
    RuntimeTarget {
        address: TerminalAddress {
            hub_id: origin(),
            terminal_id: TerminalId::from_uuid(Uuid::from_u128(SESSION_VALUE)).unwrap(),
        },
        runtime_epoch: RuntimeEpoch::from_uuid(Uuid::from_u128(EPOCH_VALUE)).unwrap(),
    }
}
pub fn principal() -> WriterPrincipal {
    WriterPrincipal {
        client_id: Uuid::from_u128(CLIENT_VALUE),
        controller_instance_id: Uuid::from_u128(INSTANCE_VALUE),
    }
}
pub fn grant() -> LeaseGrant {
    LeaseGrant::new(LeaseGrantFields {
        target: target(),
        principal: principal(),
        generation: GENERATION,
        capability: SecretCapability::generate(),
        expires_at_ms: EXPIRES_MS,
        ttl_ms: TTL_MS,
        renew_after_ms: RENEW_MS,
    })
    .unwrap()
}
pub fn initial() -> AttachResult {
    AttachResult::new(AttachResultFields {
        target: target(),
        snapshot: Some(
            TerminalSnapshot::new(TerminalSnapshotFields {
                target: target(),
                barrier_sequence: ZERO_SEQUENCE,
                size: TerminalSize::default(),
                data: SNAPSHOT.to_vec(),
            })
            .unwrap(),
        ),
        chunks: vec![],
        history: None,
        next_sequence: FIRST_OUTPUT_SEQUENCE,
        resynchronized: true,
    })
    .unwrap()
}
pub fn deadline() -> Instant {
    Instant::now() + TEST_TIMEOUT
}
