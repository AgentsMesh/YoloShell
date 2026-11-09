use super::*;
use crate::agent::{AgentKind, AgentRecoveryAvailability, AgentState};
use tempfile::tempdir;
use uuid::Uuid;

const AGENT_SESSION_ID: &str = "550e8400-e29b-41d4-a716-446655440000";
const DATA_HOME_DIRECTORY: &str = "codex-home";
const TEST_TIMESTAMP_MS: i64 = 1;

#[test]
fn save_preserves_the_exact_binding_payload() {
    let directory = tempdir().unwrap();
    let path = directory
        .path()
        .join(crate::agent::constants::BINDING_FILE_NAME);
    crate::paths::ensure_private_parent(&path).unwrap();
    let data_home = directory.path().join(DATA_HOME_DIRECTORY);
    let binding = AgentBinding {
        schema_version: crate::agent::constants::BINDING_SCHEMA_VERSION,
        terminal_id: Uuid::new_v4(),
        kind: AgentKind::Codex,
        provider_session_id: AGENT_SESSION_ID.into(),
        executable: crate::agent::codex::constants::EXECUTABLE.into(),
        cwd: directory.path().display().to_string(),
        evidence_path: data_home
            .join(crate::agent::codex::constants::EVIDENCE_DIR)
            .display()
            .to_string(),
        data_home: data_home.display().to_string(),
        process_pid: None,
        process_start_time: None,
        state: AgentState::Exited,
        recovery_availability: AgentRecoveryAvailability::Available,
        detected_at_ms: TEST_TIMESTAMP_MS,
        updated_at_ms: TEST_TIMESTAMP_MS,
    };
    let store = AgentBindingStore::new(path);

    store.save(&binding).unwrap();

    assert_eq!(
        crate::paths::read_json::<AgentBinding>(store.path()).unwrap(),
        binding
    );
}
