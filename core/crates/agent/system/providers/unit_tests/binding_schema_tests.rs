use super::binding_schema_test_constants::{
    FUTURE_AGENT_STATE_WIRE, PERMISSION_STATE_WIRE, WAITING_INPUT_STATE_WIRE,
};
use super::*;
use serde_json::json;
use std::path::Path;
use tempfile::tempdir;
use uuid::Uuid;
use yolo_agent_adapter_contract::DetectionContext;
use yolo_agent_contract::{AgentBinding, AgentRecoveryAvailability, AgentState};

const PROCESS_PID: u32 = 19;
const PROCESS_START: u64 = 23;
const AGENT_SESSION: &str = "550e8400-e29b-41d4-a716-446655440000";
const EVIDENCE_DATE_DIRECTORY: &str = "2026/08/05";
const EVIDENCE_FILE: &str =
    "rollout-2026-08-05T12-00-00-550e8400-e29b-41d4-a716-446655440000.jsonl";
struct Fixture {
    _directory: tempfile::TempDir,
    context: DetectionContext,
    binding: AgentBinding,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempdir().unwrap();
        let root = directory.path().canonicalize().unwrap();
        let cwd = root.join("project");
        let data_home = root.join("codex-home");
        let evidence = data_home
            .join(yolo_codex::codex::constants::EVIDENCE_DIR)
            .join(EVIDENCE_DATE_DIRECTORY)
            .join(EVIDENCE_FILE);
        std::fs::create_dir_all(&cwd).unwrap();
        std::fs::create_dir_all(evidence.parent().unwrap()).unwrap();
        write_evidence(&evidence, &cwd);
        let session_id = Uuid::new_v4();
        let context = DetectionContext::new(
            session_id,
            cwd.clone(),
            [(AgentKind::Codex, data_home.clone())],
        );
        let binding = AgentBinding {
            schema_version: constants::BINDING_SCHEMA_VERSION,
            terminal_id: session_id,
            kind: AgentKind::Codex,
            provider_session_id: AGENT_SESSION.into(),
            executable: yolo_codex::codex::constants::EXECUTABLE.into(),
            cwd: cwd.display().to_string(),
            evidence_path: evidence.display().to_string(),
            data_home: data_home.display().to_string(),
            process_pid: Some(PROCESS_PID),
            process_start_time: Some(PROCESS_START),
            state: AgentState::Running,
            recovery_availability: AgentRecoveryAvailability::Available,
            detected_at_ms: 1,
            updated_at_ms: 2,
        };
        Self {
            _directory: directory,
            context,
            binding,
        }
    }
}

fn write_evidence(path: &Path, cwd: &Path) {
    let record = json!({
        (yolo_codex::codex::constants::RECORD_TYPE_KEY): yolo_codex::codex::constants::SESSION_META_TYPE,
        (yolo_codex::codex::constants::PAYLOAD_KEY): {
            (yolo_codex::codex::constants::SESSION_ID_KEY): AGENT_SESSION,
            (yolo_codex::codex::constants::CWD_KEY): cwd,
            (yolo_codex::codex::constants::SOURCE_KEY): yolo_codex::codex::constants::CLI_SOURCE,
        },
    });
    std::fs::write(path, format!("{record}{}", constants::COMMAND_LINE_ENDING)).unwrap();
}

#[test]
fn process_identity_round_trips_for_running_and_exited_bindings() {
    let fixture = Fixture::new();
    let registry = builtin_agent_registry();
    registry
        .validate_binding(&fixture.binding, &fixture.context)
        .unwrap();
    let value = serde_json::to_value(&fixture.binding).unwrap();
    assert_eq!(value[constants::PROCESS_PID_FIELD], PROCESS_PID);
    assert_eq!(value[constants::PROCESS_START_TIME_FIELD], PROCESS_START);
    assert_eq!(
        serde_json::from_value::<AgentBinding>(value).unwrap(),
        fixture.binding
    );

    let mut exited = fixture.binding;
    exited.state = AgentState::Exited;
    exited.process_pid = None;
    exited.process_start_time = None;
    exited.recovery_availability = AgentRecoveryAvailability::Unavailable;
    registry
        .validate_binding(&exited, &fixture.context)
        .unwrap();
    let value = serde_json::to_value(&exited).unwrap();
    assert!(value[constants::PROCESS_PID_FIELD].is_null());
    assert!(value[constants::PROCESS_START_TIME_FIELD].is_null());
    assert_eq!(
        serde_json::from_value::<AgentBinding>(value).unwrap(),
        exited
    );
}

#[test]
fn permission_waiting_state_round_trips_as_recoverable_hook_state() {
    let mut fixture = Fixture::new();
    fixture.binding.state = AgentState::WaitingForPermission;
    builtin_agent_registry()
        .validate_binding(&fixture.binding, &fixture.context)
        .unwrap();

    let value = serde_json::to_value(&fixture.binding).unwrap();
    assert_eq!(
        value[constants::STATE_FIELD],
        serde_json::json!(PERMISSION_STATE_WIRE)
    );
    assert_eq!(
        serde_json::from_value::<AgentBinding>(value).unwrap(),
        fixture.binding
    );
}

#[test]
fn agent_state_wire_keeps_prior_values_and_rejects_unknown_future_values() {
    let fixture = Fixture::new();
    let mut value = serde_json::to_value(&fixture.binding).unwrap();
    value[constants::STATE_FIELD] = serde_json::json!(WAITING_INPUT_STATE_WIRE);
    let prior: AgentBinding = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(prior.state, AgentState::WaitingForInput);

    value[constants::STATE_FIELD] = serde_json::json!(FUTURE_AGENT_STATE_WIRE);
    assert!(serde_json::from_value::<AgentBinding>(value).is_err());
}

#[test]
fn partial_process_identity_and_retired_schema_fail_closed() {
    let fixture = Fixture::new();
    let registry = builtin_agent_registry();
    let mut missing_pid = fixture.binding.clone();
    missing_pid.process_pid = None;
    assert!(
        registry
            .validate_binding(&missing_pid, &fixture.context)
            .is_err()
    );

    let mut missing_start = fixture.binding.clone();
    missing_start.process_start_time = None;
    assert!(
        registry
            .validate_binding(&missing_start, &fixture.context)
            .is_err()
    );

    let mut retired = fixture.binding;
    retired.schema_version = constants::BINDING_SCHEMA_VERSION.saturating_sub(1);
    let error = registry
        .validate_binding(&retired, &fixture.context)
        .unwrap_err();
    assert!(error.to_string().contains(constants::validation::SCHEMA));

    let mut value = serde_json::to_value(&retired).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .remove(constants::PROCESS_START_TIME_FIELD);
    value[constants::SCHEMA_VERSION_FIELD] = constants::BINDING_SCHEMA_VERSION.into();
    let missing_start: AgentBinding = serde_json::from_value(value).unwrap();
    let error = registry
        .validate_binding(&missing_start, &fixture.context)
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains(constants::validation::PROCESS_START_TIME)
    );
}
