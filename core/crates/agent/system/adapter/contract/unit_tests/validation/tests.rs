use super::*;
use crate::constants::BINDING_SCHEMA_VERSION;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;
use uuid::Uuid;
use yolo_agent_contract::AgentRecoveryAvailability;

const AGENT_SESSION_ID: &str = "agent-session";
const PROCESS_PID: u32 = 41;
const DETECTED_AT_MS: i64 = 10;
const UPDATED_AT_MS: i64 = 20;
const TEST_EXECUTABLE: &str = "test-agent";

fn fixture() -> (tempfile::TempDir, DetectionContext, AgentBinding, PathBuf) {
    let directory = tempdir().unwrap();
    let cwd = directory.path().join("project");
    let home = directory.path().join("agent-home");
    let root = home.join("sessions");
    fs::create_dir_all(&cwd).unwrap();
    fs::create_dir_all(&root).unwrap();
    let evidence = root.join("evidence.jsonl");
    fs::write(&evidence, b"{}\n").unwrap();
    let session_id = Uuid::new_v4();
    let context =
        DetectionContext::new(session_id, cwd.clone(), [(AgentKind::Codex, home.clone())]);
    let binding = AgentBinding {
        schema_version: BINDING_SCHEMA_VERSION,
        terminal_id: session_id,
        kind: AgentKind::Codex,
        provider_session_id: AGENT_SESSION_ID.into(),
        executable: TEST_EXECUTABLE.into(),
        cwd: cwd.display().to_string(),
        evidence_path: evidence.display().to_string(),
        data_home: home.display().to_string(),
        process_pid: Some(PROCESS_PID),
        process_start_time: Some(u64::from(PROCESS_PID)),
        state: AgentState::Running,
        recovery_availability: AgentRecoveryAvailability::Available,
        detected_at_ms: DETECTED_AT_MS,
        updated_at_ms: UPDATED_AT_MS,
    };
    (directory, context, binding, root)
}

fn rejected(binding: &AgentBinding, context: &DetectionContext, expected: &str) {
    let error = validate_core(binding, context).unwrap_err();
    assert!(error.to_string().contains(expected), "{error:#}");
}

#[path = "binding_tests.rs"]
mod binding_tests;
#[path = "evidence_tests.rs"]
mod evidence_tests;
#[path = "recovery_availability_tests.rs"]
mod recovery_availability_tests;
