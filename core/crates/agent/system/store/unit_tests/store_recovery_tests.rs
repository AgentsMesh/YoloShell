use super::*;
use crate::agent::constants::BINDING_SCHEMA_VERSION;
use yolo_agent_launch::{ResumeIntent, ResumePolicy};
use serde_json::json;
use std::fs;
use tempfile::tempdir;
use uuid::Uuid;

const AGENT_SESSION_ID: &str = "550e8400-e29b-41d4-a716-446655440000";
const CODEX_EVIDENCE_DIRECTORY: &str = "2026/08/05";
const CODEX_EVIDENCE_FILE: &str =
    "rollout-2026-08-05T12-00-00-550e8400-e29b-41d4-a716-446655440000.jsonl";
const HOST_CODEX_HOME_DIRECTORY: &str = "host-codex-home";
const DETECTED_AT_MS: i64 = 100;
const UPDATED_AT_MS: i64 = 200;
const INVALID_EXITED_PID: u32 = 501;
const CLAUDE_SESSION_ID: &str = "c59af239-7fbd-47da-bca2-42d6de7075ab";
const CLAUDE_PROJECT_DIRECTORY: &str = "claude-project";
const CLAUDE_HOME_DIRECTORY: &str = "claude-home";
const CLAUDE_SCOPE_DIRECTORY: &str = "project-scope";
const BINDING_FILE: &str = "binding.json";
const CLAUDE_PROCESS_PID: u32 = 502;
const CLAUDE_PROCESS_START: u64 = 503;

#[test]
fn exited_custom_home_binding_fails_closed_without_hook_authority() {
    let directory = tempdir().unwrap();
    let root = directory.path().canonicalize().unwrap();
    let shell_cwd = root.join("shell-launch");
    let agent_cwd = root.join("agent-project");
    let codex_home = root.join("codex-home");
    let host_codex_home = root.join(HOST_CODEX_HOME_DIRECTORY);
    let evidence = codex_home
        .join(codex::constants::EVIDENCE_DIR)
        .join(CODEX_EVIDENCE_DIRECTORY)
        .join(CODEX_EVIDENCE_FILE);
    fs::create_dir_all(&shell_cwd).unwrap();
    fs::create_dir_all(&agent_cwd).unwrap();
    fs::create_dir_all(&host_codex_home).unwrap();
    fs::create_dir_all(evidence.parent().unwrap()).unwrap();
    let terminal_id = Uuid::new_v4();
    let binding = AgentBinding {
        schema_version: BINDING_SCHEMA_VERSION,
        terminal_id,
        kind: AgentKind::Codex,
        provider_session_id: AGENT_SESSION_ID.into(),
        executable: codex::constants::EXECUTABLE.into(),
        cwd: agent_cwd.display().to_string(),
        evidence_path: evidence.display().to_string(),
        data_home: codex_home.display().to_string(),
        process_pid: None,
        process_start_time: None,
        state: AgentState::Exited,
        recovery_availability: crate::agent::AgentRecoveryAvailability::Available,
        detected_at_ms: DETECTED_AT_MS,
        updated_at_ms: UPDATED_AT_MS,
    };
    write_codex_evidence(&binding);
    let context = DetectionContext::new(
        terminal_id,
        shell_cwd,
        [(AgentKind::Codex, host_codex_home)],
    );
    let store = AgentBindingStore::new(directory.path().join("binding.json"));
    store.save(&binding).unwrap();
    let loaded = store.load(&context).unwrap().unwrap();
    assert_eq!(loaded.state, AgentState::Exited);
    assert_eq!(
        loaded.recovery_availability,
        crate::agent::AgentRecoveryAvailability::Unavailable
    );
    assert_eq!(loaded.provider_session_id, binding.provider_session_id);
    for intent in [ResumeIntent::Automatic, ResumeIntent::Explicit] {
        assert_eq!(intent.policy(&loaded), ResumePolicy::FailClosed);
    }

    let mut invalid = binding;
    invalid.process_pid = Some(INVALID_EXITED_PID);
    store.save(&invalid).unwrap();
    assert!(
        store
            .load(&context)
            .unwrap_err()
            .to_string()
            .contains(constants::validation::PROCESS_PID)
    );
}

#[test]
fn unscoped_claude_load_fails_closed_without_runtime_authority() {
    let directory = tempdir().unwrap();
    let cwd = directory.path().join(CLAUDE_PROJECT_DIRECTORY);
    let data_home = directory.path().join(CLAUDE_HOME_DIRECTORY);
    let evidence = data_home
        .join(claude::constants::EVIDENCE_DIR)
        .join(CLAUDE_SCOPE_DIRECTORY)
        .join(format!(
            "{CLAUDE_SESSION_ID}.{}",
            constants::JSONL_EXTENSION
        ));
    fs::create_dir_all(&cwd).unwrap();
    fs::create_dir_all(evidence.parent().unwrap()).unwrap();
    let record = json!({
        (claude::constants::RECORD_TYPE_KEY): claude::constants::USER_RECORD,
        (claude::constants::SESSION_ID_KEY): CLAUDE_SESSION_ID,
        (claude::constants::CWD_KEY): cwd,
    });
    fs::write(&evidence, format!("{record}\n")).unwrap();
    let terminal_id = Uuid::new_v4();
    let binding = AgentBinding {
        schema_version: BINDING_SCHEMA_VERSION,
        terminal_id,
        kind: AgentKind::ClaudeCode,
        provider_session_id: CLAUDE_SESSION_ID.into(),
        executable: claude::constants::EXECUTABLE.into(),
        cwd: cwd.display().to_string(),
        evidence_path: evidence.display().to_string(),
        data_home: data_home.display().to_string(),
        process_pid: Some(CLAUDE_PROCESS_PID),
        process_start_time: Some(CLAUDE_PROCESS_START),
        state: AgentState::Running,
        recovery_availability: crate::agent::AgentRecoveryAvailability::Available,
        detected_at_ms: DETECTED_AT_MS,
        updated_at_ms: UPDATED_AT_MS,
    };
    let context = DetectionContext::new(terminal_id, cwd, [(AgentKind::ClaudeCode, data_home)]);
    let store = AgentBindingStore::new(directory.path().join(BINDING_FILE));
    store.save(&binding).unwrap();
    let loaded = store.load(&context).unwrap().unwrap();
    assert_eq!(loaded.state, AgentState::Unknown);
    assert_eq!(
        loaded.recovery_availability,
        crate::agent::AgentRecoveryAvailability::Unavailable
    );
    assert_eq!(
        ResumeIntent::Automatic.policy(&loaded),
        ResumePolicy::FailClosed
    );
}

fn write_codex_evidence(binding: &AgentBinding) {
    let record = json!({
        (codex::constants::RECORD_TYPE_KEY): codex::constants::SESSION_META_TYPE,
        (codex::constants::PAYLOAD_KEY): {
            (codex::constants::SESSION_ID_KEY): binding.provider_session_id,
            (codex::constants::CWD_KEY): binding.cwd,
            (codex::constants::SOURCE_KEY): codex::constants::CLI_SOURCE,
        },
    });
    fs::write(
        &binding.evidence_path,
        format!("{record}{}", constants::COMMAND_LINE_ENDING),
    )
    .unwrap();
}
