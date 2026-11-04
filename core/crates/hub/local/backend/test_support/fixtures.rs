use uuid::Uuid;
use yolo_hub_identity::{HubId, RuntimeEpoch, RuntimeTarget, TerminalAddress, TerminalId};
use yolo_hub_terminal_protocol::{TerminalAction, TerminalCommand, TerminalSelection, WriterPrincipal};
use yolo_runtime::{model::*, protocol::EventEnvelope};

#[path = "constants.rs"]
pub mod constants;
use constants::*;
#[path = "agent_control.rs"]
pub mod agent_control;

pub fn target() -> RuntimeTarget {
    RuntimeTarget {
        address: TerminalAddress {
            hub_id: HubId::from_uuid(Uuid::from_u128(HUB_VALUE)).unwrap(),
            terminal_id: TerminalId::from_uuid(Uuid::from_u128(SESSION_VALUE)).unwrap(),
        },
        runtime_epoch: RuntimeEpoch::from_uuid(Uuid::from_u128(EPOCH_VALUE)).unwrap(),
    }
}

pub fn principal() -> WriterPrincipal {
    WriterPrincipal {
        client_id: Uuid::from_u128(CLIENT_VALUE),
        controller_instance_id: Uuid::from_u128(CONTROLLER_VALUE),
    }
}

pub fn grant() -> WriterLeaseGrant {
    WriterLeaseGrant {
        lease: WriterLease {
            holder_id: principal().client_id.to_string(),
            controller_instance_id: Some(principal().controller_instance_id.to_string()),
            generation: yolo_hub_terminal_protocol::FIRST_LEASE_GENERATION,
            expires_at_ms: EXPIRY,
            ttl_ms: TTL,
            renew_after_ms: RENEW,
        },
        writer_capability: WriterCapability::generate(),
    }
}

pub fn view() -> TerminalView {
    TerminalView {
        manifest: TerminalManifest {
            schema_version: SCHEMA,
            terminal_id: target().address.terminal_id.as_uuid(),
            name: NAME.into(),
            command: SHELL.into(),
            args: Vec::new(),
            cwd: DIRECTORY.into(),
            cols: DEFAULT_COLS,
            rows: DEFAULT_ROWS,
            created_at_ms: CLOCK,
            updated_at_ms: CLOCK,
            archived: false,
            recovery_state: RecoveryState::Created,
        },
        runtime: Some(RuntimeView {
            presentation: Default::default(),
            runtime_epoch: target().runtime_epoch.as_uuid(),
            worker_id: Uuid::new_v4(),
            worker_pid: PROCESS_ID,
            worker_start_time: PROCESS_START,
            child_pid: PROCESS_ID,
            child_start_time: PROCESS_START,
            status: TerminalRuntimeStatus::Running,
            last_output_sequence: SEQUENCE,
            exit: None,
            bootstrap_delivery: BootstrapDelivery::NotRequired,
            capabilities: None,
            fault_reap: None,
        }),
        agent: None,
        completed_attachment: None,
    }
}

pub fn command() -> TerminalCommand {
    TerminalCommand {
        request_id: Uuid::new_v4(),
        selection: TerminalSelection {
            address: target().address,
            expected_runtime: Some(target().runtime_epoch),
        },
        action: TerminalAction::Archive {},
    }
}

pub fn output() -> OutputChunk {
    OutputChunk {
        runtime_epoch: target().runtime_epoch.as_uuid(),
        sequence: SEQUENCE,
        data: OUTPUT.to_vec(),
    }
}

pub fn exit() -> ExitRecord {
    ExitRecord {
        runtime_epoch: target().runtime_epoch.as_uuid(),
        code: Some(0),
        signal: None,
        reason: ExitReason::Natural,
        observed_at_ms: CLOCK,
        last_output_sequence: SEQUENCE,
    }
}

pub fn event<T: serde::Serialize>(name: &str, payload: &T) -> EventEnvelope {
    EventEnvelope::new(
        name,
        Some(target().address.terminal_id.to_string()),
        Some(target().runtime_epoch.to_string()),
        payload,
    )
    .unwrap()
}
