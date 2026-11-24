use super::{grant, principal, target};
use uuid::Uuid;
use yolo_hub_agent_protocol::*;
use yolo_hub_identity::{AgentAddress, AgentId, AgentTarget, ExecutionId};
use yolo_hub_terminal_protocol::LeaseGrant;

pub const PROVIDER: &str = "claude-code";
pub const OBSERVATION_TIME: u64 = 1;
pub const SINGLE_AGENT: usize = 1;

pub fn snapshot() -> AgentSnapshot {
    let terminal = target();
    let target = AgentTarget {
        address: AgentAddress {
            hub_id: terminal.address.hub_id,
            agent_id: AgentId::new(),
        },
        execution_id: ExecutionId::new(),
    };
    AgentSnapshot::new(AgentSnapshotFields {
        schema_version: AGENT_PROTOCOL_SCHEMA_VERSION,
        cursor: ObservationCursor::new(target, Uuid::new_v4(), FIRST_OBSERVATION_SEQUENCE).unwrap(),
        terminal,
        provider: ObservedProvider::new(PROVIDER.into(), None, None).unwrap(),
        working_directory: None,
        lifecycle: Lifecycle::Running {},
        activity: Activity::Working,
        reachability: Reachability::Reachable,
        freshness: Freshness::Current,
        source: ObservationSource::ProviderHook,
        observed_unix_ms: OBSERVATION_TIME,
        recovery: RecoveryStatus::Available,
        capabilities: AgentCapabilities::UNSUPPORTED,
        interaction: None,
    })
    .unwrap()
}

pub fn request(snapshot: &AgentSnapshot) -> AgentControlRequest {
    AgentControlRequest::new(AgentControlFields {
        schema_version: AGENT_PROTOCOL_SCHEMA_VERSION,
        request_id: Uuid::new_v4(),
        target: snapshot.cursor().target(),
        expected_cursor: snapshot.cursor(),
        action: AgentAction::Interrupt {},
    })
    .unwrap()
}

pub fn authority() -> LeaseGrant {
    crate::writer_conversion::public_grant(target(), principal(), grant()).unwrap()
}

pub fn host_view() -> yolo_runtime::model::TerminalView {
    use super::{constants::*, view};
    use yolo_agent_contract::{AgentBinding, AgentKind, AgentRecoveryAvailability, AgentState};
    use yolo_runtime::agent::adapter::constants::BINDING_SCHEMA_VERSION;
    let mut view = view();
    view.agent = Some(AgentBinding {
        schema_version: BINDING_SCHEMA_VERSION,
        terminal_id: view.manifest.terminal_id,
        kind: AgentKind::Codex,
        provider_session_id: NAME.into(),
        executable: SHELL.into(),
        cwd: DIRECTORY.into(),
        evidence_path: DIRECTORY.into(),
        data_home: DIRECTORY.into(),
        process_pid: Some(PROCESS_ID),
        process_start_time: Some(PROCESS_START),
        state: AgentState::Running,
        recovery_availability: AgentRecoveryAvailability::Available,
        detected_at_ms: CLOCK,
        updated_at_ms: CLOCK,
    });
    view
}
