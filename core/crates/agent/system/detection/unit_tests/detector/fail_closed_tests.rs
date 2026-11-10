use super::*;
use crate::agent::AgentEvidence;
use crate::agent::claude::constants as claude;
use crate::agent::launch::{ResumeIntent, apply_resume_with_registry};
use crate::application_constants::WORKER_LAUNCH_SCHEMA_VERSION;
use crate::model::{DEFAULT_COLS, DEFAULT_ROWS, WorkerLaunchSpec};
use serde_json::{Value, json};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use tempfile::tempdir;
use uuid::Uuid;

const PROJECT_DIRECTORY: &str = "project";
const CLAUDE_HOME_DIRECTORY: &str = "claude-home";
const EVIDENCE_FILE: &str = "772a8ed6-b04e-4639-a25b-923fab4d46ad.jsonl";
const SESSION_ID: &str = "772a8ed6-b04e-4639-a25b-923fab4d46ad";
const PROJECT_SCOPE: &str = "project-scope";
const MALFORMED_CONTENT: &str = "content-must-be-an-array";
const TOOL_ID: &str = "tool-id";
const TOOL_NAME: &str = "Bash";
const WORKER_SOCKET_FILE: &str = "worker.sock";
const AUTH_TOKEN: &str = "test-auth-token";
const SHELL_EXECUTABLE: &str = "/bin/sh";
const ABSOLUTE_CLAUDE_EXECUTABLE: &str = "/opt/homebrew/bin/claude";
const EXPECTED_BINDING: &str = "Claude evidence must create one binding";
const TEXT_VALUE: &str = "ready";
const PROCESS_PID: u32 = 707;
const PARENT_PID: u32 = 1;
const PROCESS_GROUP: i32 = 97;
const CONTROLLER_GENERATION: u64 = 1;

#[test]
fn provider_exited_evidence_remains_exited_without_process_identity() {
    let directory = tempdir().unwrap();
    let cwd = directory.path();
    let context = DetectionContext::new(
        Uuid::new_v4(),
        cwd.into(),
        [(AgentKind::ClaudeCode, cwd.into())],
    );
    let registry = AgentRegistry::from_adapters(Vec::new()).unwrap();
    let mut detector = AgentDetector::new(context, registry, None).unwrap();
    let evidence = AgentEvidence {
        provider_session_id: SESSION_ID.into(),
        cwd: cwd.into(),
        path: cwd.join(EVIDENCE_FILE),
        data_home: cwd.into(),
        state: AgentState::Exited,
    };
    let result = detector.apply_candidate(AgentKind::ClaudeCode, &claude_process(cwd), evidence);
    assert!(matches!(
        result,
        DetectionResult::BindingChanged(binding)
            if binding.state == AgentState::Exited
                && binding.process_pid.is_none()
                && binding.process_start_time.is_none()
    ));
}

#[test]
fn malformed_claude_evidence_cannot_become_automatically_resumable() {
    assert_claude_evidence_fails_closed(malformed_record);
}

#[test]
fn conflicting_claude_evidence_cannot_become_automatically_resumable() {
    assert_claude_evidence_fails_closed(conflicting_record);
}

#[test]
fn unshimmed_absolute_claude_with_valid_evidence_cannot_be_automatically_resumed() {
    assert_claude_evidence_fails_closed(valid_record);
}

fn assert_claude_evidence_fails_closed(record: fn(&Path) -> Value) {
    let directory = tempdir().unwrap();
    let cwd = directory.path().join(PROJECT_DIRECTORY);
    let data_home = directory.path().join(CLAUDE_HOME_DIRECTORY);
    let evidence_directory = data_home.join(claude::EVIDENCE_DIR).join(PROJECT_SCOPE);
    std::fs::create_dir_all(&cwd).unwrap();
    std::fs::create_dir_all(&evidence_directory).unwrap();
    let evidence = evidence_directory.join(EVIDENCE_FILE);
    write_record(&evidence, &record(&cwd));
    let context = DetectionContext::new(
        Uuid::new_v4(),
        cwd.clone(),
        [(AgentKind::ClaudeCode, data_home)],
    );
    let mut detector = AgentDetector::new(context, yolo_agent_providers::builtin_agent_registry(), None).unwrap();
    let mut process = claude_process(&cwd);
    process.executable = ABSOLUTE_CLAUDE_EXECUTABLE.to_owned();
    process.argv[0] = ABSOLUTE_CLAUDE_EXECUTABLE.to_owned();
    process.evidence_paths = Some(vec![evidence]);
    let DetectionResult::BindingChanged(binding) = detector.detect(&[process]).unwrap() else {
        panic!("{EXPECTED_BINDING}")
    };
    assert_eq!(binding.state, AgentState::Unknown);
    assert_eq!(
        binding.recovery_availability,
        crate::agent::AgentRecoveryAvailability::UnmanagedLaunch
    );
    let mut launch = worker_launch(&cwd);
    let unchanged = launch.clone();
    let registry = yolo_agent_providers::builtin_agent_registry();
    for intent in [ResumeIntent::Automatic, ResumeIntent::Explicit] {
        assert!(apply_resume_with_registry(&mut launch, &binding, intent, &registry).is_err());
    }
    assert_eq!(launch, unchanged);
}

fn valid_record(cwd: &Path) -> Value {
    claude_record(
        cwd,
        json!({
            (claude::CONTENT_KEY): [{
                (claude::CONTENT_TYPE_KEY): claude::TEXT_CONTENT,
                (claude::TEXT_VALUE_KEY): TEXT_VALUE,
            }],
            (claude::STOP_REASON_KEY): claude::END_TURN_REASON,
        }),
    )
}

fn write_record(path: &Path, record: &Value) {
    let mut file = File::create(path).unwrap();
    serde_json::to_writer(&mut file, record).unwrap();
    writeln!(file).unwrap();
}

fn malformed_record(cwd: &Path) -> Value {
    claude_record(
        cwd,
        json!({
            (claude::CONTENT_KEY): MALFORMED_CONTENT,
            (claude::STOP_REASON_KEY): claude::END_TURN_REASON,
        }),
    )
}

fn conflicting_record(cwd: &Path) -> Value {
    claude_record(
        cwd,
        json!({
            (claude::CONTENT_KEY): [{
                (claude::CONTENT_TYPE_KEY): claude::TOOL_USE_CONTENT,
                (claude::TOOL_ID_KEY): TOOL_ID,
                (claude::TOOL_NAME_KEY): TOOL_NAME,
                (claude::TOOL_INPUT_KEY): {},
            }],
            (claude::STOP_REASON_KEY): claude::END_TURN_REASON,
        }),
    )
}

fn claude_record(cwd: &Path, message: Value) -> Value {
    json!({
        (claude::RECORD_TYPE_KEY): claude::ASSISTANT_RECORD,
        (claude::SESSION_ID_KEY): SESSION_ID,
        (claude::CWD_KEY): cwd.display().to_string(),
        (claude::MESSAGE_KEY): message,
    })
}

fn claude_process(cwd: &Path) -> ObservedProcess {
    ObservedProcess {
        pid: PROCESS_PID,
        start_time: u64::from(PROCESS_PID),
        parent_pid: PARENT_PID,
        process_group: PROCESS_GROUP,
        foreground_group: PROCESS_GROUP,
        executable: claude::EXECUTABLE.into(),
        argv: vec![claude::EXECUTABLE.into()],
        environment: None,
        cwd: Some(cwd.into()),
        evidence_paths: None,
    }
}

fn worker_launch(cwd: &Path) -> WorkerLaunchSpec {
    WorkerLaunchSpec {
        schema_version: WORKER_LAUNCH_SCHEMA_VERSION,
        terminal_id: Uuid::new_v4(),
        runtime_epoch: Uuid::new_v4(),
        worker_id: Uuid::new_v4(),
        socket_path: cwd.join(WORKER_SOCKET_FILE),
        auth_token: AUTH_TOKEN.into(),
        controller_generation: CONTROLLER_GENERATION,
        command: SHELL_EXECUTABLE.into(),
        args: Vec::new(),
        cwd: cwd.into(),
        environment: Default::default(),
        initial_input: None,
        cols: DEFAULT_COLS,
        rows: DEFAULT_ROWS,
        context: None,
    }
}
