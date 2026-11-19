use super::*;
use tempfile::tempdir;
use uuid::Uuid;

const BINDING_SESSION_ID: &str = "fb9bed41-d721-4fb5-95d9-2f3594d5ff6f";
const BINDING_PROJECT_DIRECTORY: &str = "project";
const BINDING_DATA_HOME_DIRECTORY: &str = "claude-home";
const BINDING_SCOPE_DIRECTORY: &str = "project-scope";
const BINDING_EVIDENCE_NAME: &str = "fb9bed41-d721-4fb5-95d9-2f3594d5ff6f.jsonl";
const BINDING_EXECUTABLE_DIRECTORY: &str = "/usr/local/bin";
const BINDING_STUB_JSON: &[u8] = b"{}";
const BINDING_PROCESS_PID: u32 = 49_001;
const BINDING_PROCESS_START: u64 = 49_002;
const BINDING_DETECTED_AT: i64 = 49_003;
const BINDING_UPDATED_AT: i64 = 49_004;
const INVALID_EXECUTABLE: &str = "not-claude";
const RELATIVE_WORKING_DIRECTORY: &str = "relative-project";

#[test]
fn claude_binding_validation_covers_identity_and_path_policy() {
    let directory = tempdir().unwrap();
    let cwd = directory.path().join(BINDING_PROJECT_DIRECTORY);
    let data_home = directory.path().join(BINDING_DATA_HOME_DIRECTORY);
    let evidence_root = data_home.join(constants::EVIDENCE_DIR);
    let evidence = evidence_root
        .join(BINDING_SCOPE_DIRECTORY)
        .join(BINDING_EVIDENCE_NAME);
    std::fs::create_dir_all(&cwd).unwrap();
    std::fs::create_dir_all(evidence.parent().unwrap()).unwrap();
    std::fs::write(&evidence, BINDING_STUB_JSON).unwrap();
    let session_id = Uuid::new_v4();
    let context = DetectionContext::new(
        session_id,
        cwd.clone(),
        [(AgentKind::ClaudeCode, data_home.clone())],
    );
    let binding = AgentBinding {
        schema_version: super::super::constants::BINDING_SCHEMA_VERSION,
        terminal_id: session_id,
        kind: AgentKind::ClaudeCode,
        provider_session_id: BINDING_SESSION_ID.to_owned(),
        executable: Path::new(BINDING_EXECUTABLE_DIRECTORY)
            .join(constants::EXECUTABLE)
            .display()
            .to_string(),
        cwd: cwd.display().to_string(),
        evidence_path: evidence.display().to_string(),
        data_home: data_home.display().to_string(),
        process_pid: Some(BINDING_PROCESS_PID),
        process_start_time: Some(BINDING_PROCESS_START),
        state: AgentState::Running,
        recovery_availability: crate::agent::AgentRecoveryAvailability::Available,
        detected_at_ms: BINDING_DETECTED_AT,
        updated_at_ms: BINDING_UPDATED_AT,
    };
    let adapter = ClaudeAdapter;
    adapter.validate_binding(&binding, &context).unwrap();
    let mut node = binding.clone();
    node.executable = super::super::constants::NODE_EXECUTABLES
        .first()
        .unwrap()
        .to_string();
    adapter.validate_binding(&node, &context).unwrap();

    for invalid in invalid_bindings(binding, directory.path()) {
        assert!(adapter.validate_binding(&invalid, &context).is_err());
    }
}

fn invalid_bindings(binding: AgentBinding, outside: &Path) -> Vec<AgentBinding> {
    let mut kind = binding.clone();
    kind.kind = AgentKind::Codex;
    let mut executable = binding.clone();
    executable.executable = INVALID_EXECUTABLE.to_owned();
    let mut cwd = binding.clone();
    cwd.cwd = RELATIVE_WORKING_DIRECTORY.to_owned();
    let mut data_home = binding.clone();
    data_home.data_home.clear();
    let mut evidence = binding;
    evidence.evidence_path = outside.join(BINDING_EVIDENCE_NAME).display().to_string();
    vec![kind, executable, cwd, data_home, evidence]
}
