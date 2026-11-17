use super::constants::*;
use uuid::Uuid;
use yolo_hub_identity::{HubId, RuntimeEpoch, RuntimeTarget, TerminalAddress, TerminalId};
use yolo_hub_rpc_protocol::Command;
use yolo_hub_terminal_protocol::*;

pub fn grant(hub_id: HubId) -> LeaseGrant {
    LeaseGrant::new(LeaseGrantFields {
        target: RuntimeTarget {
            address: TerminalAddress {
                hub_id,
                terminal_id: TerminalId::new(),
            },
            runtime_epoch: RuntimeEpoch::new(),
        },
        principal: WriterPrincipal {
            client_id: Uuid::new_v4(),
            controller_instance_id: Uuid::new_v4(),
        },
        generation: FIRST_LEASE_GENERATION,
        capability: SecretCapability::generate(),
        expires_at_ms: EXPIRES_MS,
        ttl_ms: LEASE_TTL_MS,
        renew_after_ms: RENEW_AFTER_MS,
    })
    .unwrap()
}
pub fn input(grant: LeaseGrant) -> Command {
    Command::Input {
        request: InputRequest::new(InputFields {
            request_id: Uuid::new_v4(),
            grant,
            data: INPUT.to_vec(),
        })
        .unwrap(),
    }
}
