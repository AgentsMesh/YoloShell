use super::*;
use serde_json::Value;
use std::fs;
use tempfile::tempdir;
use uuid::Uuid;

const SESSION_ID: &str = "b6d25858-b696-40e2-a2a6-7c22551e49ca";
const PROCESS_PID: u32 = 202;
const PROCESS_GROUP: i32 = 61;
const COMPLETION_TEXT: &str = "Done";
const PROJECT_SCOPE: &str = "project-scope";
const RECORD_SEPARATOR: &str = "\n";

#[test]
fn claude_uses_consistent_structured_records_and_exact_resume_id() {
    let directory = tempdir().unwrap();
    let cwd = directory.path().join("project");
    fs::create_dir_all(&cwd).unwrap();
    let claude_home = directory.path().join("claude-home");
    let evidence_dir = claude_home
        .join(constants::EVIDENCE_DIR)
        .join(PROJECT_SCOPE);
    fs::create_dir_all(&evidence_dir).unwrap();
    let evidence = evidence_dir.join(format!(
        "{SESSION_ID}.{}",
        crate::agent::constants::JSONL_EXTENSION
    ));
    let records = [
        serde_json::json!({
            (constants::RECORD_TYPE_KEY): constants::USER_RECORD,
            (constants::SESSION_ID_KEY): SESSION_ID,
            (constants::CWD_KEY): cwd,
        }),
        serde_json::json!({
            (constants::RECORD_TYPE_KEY): constants::ASSISTANT_RECORD,
            (constants::SESSION_ID_KEY): SESSION_ID,
            (constants::CWD_KEY): cwd,
            (constants::MESSAGE_KEY): {
                (constants::CONTENT_KEY): [{
                    (constants::CONTENT_TYPE_KEY): constants::TEXT_CONTENT,
                    (constants::TEXT_VALUE_KEY): COMPLETION_TEXT,
                }],
                (constants::STOP_REASON_KEY): constants::END_TURN_REASON,
            },
        }),
    ];
    let contents = records
        .iter()
        .map(Value::to_string)
        .collect::<Vec<_>>()
        .join(RECORD_SEPARATOR);
    fs::write(&evidence, format!("{contents}{RECORD_SEPARATOR}")).unwrap();
    let adapter = ClaudeAdapter;
    let context = DetectionContext::new(
        Uuid::new_v4(),
        cwd.clone(),
        [(AgentKind::ClaudeCode, claude_home.clone())],
    );
    assert!(adapter.owns_evidence_path(&evidence, &context));
    assert!(!adapter.owns_evidence_path(&cwd.join("foreign.jsonl"), &context));
    let process = ObservedProcess {
        pid: PROCESS_PID,
        start_time: u64::from(PROCESS_PID),
        parent_pid: 1,
        process_group: PROCESS_GROUP,
        foreground_group: PROCESS_GROUP,
        executable: "node".into(),
        argv: vec![
            "node".into(),
            "/opt/@anthropic-ai/claude-code/cli.js".into(),
        ],
        environment: None,
        cwd: Some(cwd.clone()),
        evidence_paths: None,
    };
    assert!(adapter.matches_process(&process));
    let mut native = process.clone();
    native.executable = "/tmp/claude/bin/claude.exe".into();
    native.argv = vec!["claude".into(), "probe".into()];
    assert!(adapter.matches_process(&native));
    let mut echo = process.clone();
    echo.executable = "echo".into();
    echo.argv = vec!["echo".into(), "@anthropic-ai/claude-code".into()];
    assert!(!adapter.matches_process(&echo));
    for helper in [
        crate::claude_hooks::CLAUDE_HOOK_HELPER_ARGUMENT,
        crate::claude_hooks::CLAUDE_SETTINGS_WATCHDOG_ARGUMENT,
    ] {
        let mut process = process.clone();
        process.executable = constants::EXECUTABLE.into();
        process.argv = vec![constants::EXECUTABLE.into(), helper.into()];
        assert!(!adapter.matches_process(&process), "{helper}");
    }
    let found = adapter.inspect_evidence(&evidence, &cwd).unwrap().unwrap();
    assert_eq!(found.provider_session_id, SESSION_ID);
    assert_eq!(found.state, AgentState::Unknown);

    let binding = AgentBinding {
        schema_version: crate::agent::constants::BINDING_SCHEMA_VERSION,
        terminal_id: Uuid::new_v4(),
        kind: AgentKind::ClaudeCode,
        provider_session_id: SESSION_ID.into(),
        executable: constants::EXECUTABLE.into(),
        cwd: cwd.display().to_string(),
        evidence_path: evidence.display().to_string(),
        data_home: claude_home.display().to_string(),
        process_pid: Some(PROCESS_PID),
        process_start_time: Some(u64::from(PROCESS_PID)),
        state: found.state,
        recovery_availability: crate::agent::AgentRecoveryAvailability::Available,
        detected_at_ms: 1,
        updated_at_ms: 1,
    };
    assert_eq!(
        adapter.resume_plan(&binding),
        ResumePlan {
            executable: constants::EXECUTABLE.into(),
            args: vec![constants::RESUME_ARGUMENT.into(), SESSION_ID.into()],
            environment: [(
                constants::DATA_HOME_ENV.to_owned(),
                claude_home.display().to_string(),
            )]
            .into(),
        }
    );
}
