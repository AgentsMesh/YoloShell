#[path = "agent.rs"]
mod agent_fixture;
#[path = "constants.rs"]
pub mod constants;
pub use agent_fixture::agent;
#[path = "agent_control.rs"]
mod agent_control_fixture;
pub use agent_control_fixture::{agent_control, agent_receipt};
use constants::*;
#[path = "workspace_fixtures.rs"]
mod workspace_fixtures;
use uuid::Uuid;
pub use workspace_fixtures::*;
#[path = "agent_run_fixtures.rs"]
mod agent_run_fixtures;
pub use agent_run_fixtures::*;
use yolo_hub_identity::{HubId, RuntimeEpoch, RuntimeTarget, TerminalAddress, TerminalId};
use yolo_hub_terminal_protocol::*;

pub fn id(value: u128) -> Uuid {
    Uuid::from_u128(value)
}
pub fn hub(value: u128) -> HubId {
    HubId::from_uuid(id(value)).unwrap()
}
pub fn address(origin: u128, session: u128) -> TerminalAddress {
    TerminalAddress {
        hub_id: hub(origin),
        terminal_id: TerminalId::from_uuid(id(session)).unwrap(),
    }
}
pub fn target() -> RuntimeTarget {
    RuntimeTarget {
        address: address(HUB_ID, SESSION_ID),
        runtime_epoch: RuntimeEpoch::from_uuid(id(RUNTIME_ID)).unwrap(),
    }
}
pub fn principal() -> WriterPrincipal {
    WriterPrincipal {
        client_id: id(CLIENT_ID),
        controller_instance_id: id(CONTROLLER_ID),
    }
}
pub fn capability() -> SecretCapability {
    CAPABILITY.to_owned().try_into().unwrap()
}
pub fn grant_fields() -> LeaseGrantFields {
    LeaseGrantFields {
        target: target(),
        principal: principal(),
        generation: FIRST_LEASE_GENERATION,
        capability: capability(),
        expires_at_ms: LEASE_EXPIRY,
        ttl_ms: LEASE_TTL,
        renew_after_ms: LEASE_RENEWAL,
    }
}
pub fn grant() -> LeaseGrant {
    LeaseGrant::new(grant_fields()).unwrap()
}

pub fn observation() -> LeaseObservation {
    LeaseObservation::new(LeaseObservationFields {
        target: target(),
        generation: FIRST_LEASE_GENERATION,
        holder_id: id(CLIENT_ID).to_string(),
        controller_instance_id: Some(id(CONTROLLER_ID).to_string()),
        expires_at_ms: LEASE_EXPIRY,
    })
    .unwrap()
}
pub fn lease() -> LeaseRequest {
    LeaseRequest {
        request_id: id(REQUEST_ID),
        target: target(),
        principal: principal(),
        mode: LeaseMode::Acquire {},
    }
}
pub fn creation() -> CreateShellResult {
    CreateShellResult {
        address: target().address,
        creation_capability: capability(),
    }
}
pub fn create() -> CreateShellRequest {
    CreateShellRequest::new(CreateShellFields {
        schema_version: TERMINAL_PROTOCOL_SCHEMA_VERSION,
        request_id: id(REQUEST_ID),
        creation_token: id(CREATION_ID),
        name: Some(SESSION_NAME.into()),
        cwd: DIRECTORY.into(),
        size: TerminalSize::default(),
    })
    .unwrap()
}
pub fn terminal_run() -> TerminalRunRequest {
    TerminalRunRequest::new(TerminalRunFields {
        schema_version: TERMINAL_PROTOCOL_SCHEMA_VERSION,
        request_id: id(REQUEST_ID),
        creation_token: id(CREATION_ID),
        name: Some(SESSION_NAME.into()),
        cwd: DIRECTORY.into(),
        size: TerminalSize::default(),
        input: b"codex\r".to_vec(),
        workspace_id: None,
    })
    .unwrap()
}
pub fn agent_start() -> yolo_hub_agent_protocol::AgentStartRequest {
    yolo_hub_agent_protocol::AgentStartRequest::new(yolo_hub_agent_protocol::AgentStartFields {
        schema_version: yolo_hub_agent_protocol::AGENT_PROTOCOL_SCHEMA_VERSION,
        request_id: id(REQUEST_ID),
        start_token: id(CREATION_ID),
        provider_id: "codex".into(),
        name: Some(SESSION_NAME.into()),
        cwd: DIRECTORY.into(),
        size: TerminalSize::default(),
        workspace_id: None,
        parent: None,
    })
    .unwrap()
}

pub fn summary(origin: u128, session: u128) -> TerminalSummary {
    TerminalSummary::new(TerminalSummaryFields {
        presentation: Default::default(),
        address: address(origin, session),
        name: SESSION_NAME.into(),
        cwd: DIRECTORY.into(),
        archived: false,
        recovery: None,
        runtime: None,
        agent: None,
        workspace: None,
    })
    .unwrap()
}
pub fn session_command() -> TerminalCommand {
    TerminalCommand {
        request_id: id(REQUEST_ID),
        selection: TerminalSelection {
            address: target().address,
            expected_runtime: Some(target().runtime_epoch),
        },
        action: TerminalAction::Terminate { force: false },
    }
}
pub fn input() -> InputRequest {
    InputRequest::new(InputFields {
        request_id: id(REQUEST_ID),
        grant: grant(),
        data: INPUT.to_vec(),
    })
    .unwrap()
}
pub fn resize() -> ResizeRequest {
    ResizeRequest {
        request_id: id(REQUEST_ID),
        grant: grant(),
        size: TerminalSize::default(),
    }
}
pub fn attach() -> AttachRequest {
    AttachRequest::new(target(), None).unwrap()
}
pub fn attached() -> AttachResult {
    AttachResult::new(AttachResultFields {
        target: target(),
        snapshot: Some(
            TerminalSnapshot::new(TerminalSnapshotFields {
                target: target(),
                barrier_sequence: FIRST_OUTPUT_SEQUENCE,
                size: TerminalSize::default(),
                data: INPUT.to_vec(),
            })
            .unwrap(),
        ),
        chunks: Vec::new(),
        history: None,
        next_sequence: FIRST_OUTPUT_SEQUENCE + STEP,
        resynchronized: true,
    })
    .unwrap()
}
