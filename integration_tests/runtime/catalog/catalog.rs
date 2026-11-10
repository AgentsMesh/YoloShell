use tempfile::tempdir;
use uuid::Uuid;
use yolo_runtime::agent::constants::{
    BINDING_SCHEMA_VERSION, FALLBACK_SHELL, LOGIN_SHELL_ARGUMENT,
};
use yolo_runtime::agent::{
    AgentBinding, AgentBindingStore, AgentKind, AgentRecoveryAvailability, AgentState,
    DetectionContext,
};
use yolo_runtime::catalog::Catalog;
use yolo_runtime::constants::SESSION_MANIFEST_SCHEMA_VERSION;
use yolo_runtime::paths::Paths;
use yolo_runtime::{RecoveryState, TerminalManifest, now_ms};

const AGENT_SESSION_ID: &str = "aac118a8-0a58-4cc6-b96d-e5301777e68d";
const AGENT_PROCESS_PID: u32 = 42;

fn fixture(id: Uuid) -> TerminalManifest {
    TerminalManifest {
        schema_version: SESSION_MANIFEST_SCHEMA_VERSION,
        terminal_id: id,
        name: "catalog integration".into(),
        command: FALLBACK_SHELL.into(),
        args: vec![LOGIN_SHELL_ARGUMENT.into()],
        cwd: std::env::current_dir().unwrap(),
        cols: 100,
        rows: 30,
        created_at_ms: now_ms(),
        updated_at_ms: now_ms(),
        archived: false,
        recovery_state: RecoveryState::Created,
    }
}

fn binding(id: Uuid, directory: &std::path::Path) -> AgentBinding {
    let data_home = directory.join("claude-home");
    AgentBinding {
        schema_version: BINDING_SCHEMA_VERSION,
        terminal_id: id,
        kind: AgentKind::ClaudeCode,
        provider_session_id: AGENT_SESSION_ID.into(),
        executable: yolo_runtime::agent::claude::constants::EXECUTABLE.into(),
        cwd: directory.to_string_lossy().into_owned(),
        evidence_path: data_home
            .join("projects/project")
            .join(AGENT_SESSION_ID)
            .with_extension(yolo_runtime::agent::constants::JSONL_EXTENSION)
            .to_string_lossy()
            .into_owned(),
        data_home: data_home.to_string_lossy().into_owned(),
        process_pid: Some(AGENT_PROCESS_PID),
        process_start_time: Some(u64::from(AGENT_PROCESS_PID)),
        state: AgentState::WaitingForInput,
        recovery_availability: AgentRecoveryAvailability::Available,
        detected_at_ms: now_ms(),
        updated_at_ms: now_ms(),
    }
}

fn binding_context(id: Uuid, directory: &std::path::Path) -> DetectionContext {
    DetectionContext::new(
        id,
        directory.into(),
        [(AgentKind::ClaudeCode, directory.join("claude-home"))],
    )
}

#[test]
fn shell_catalog_and_agent_binding_survive_reopen_without_conflation() {
    let dir = tempdir().unwrap();
    let id = Uuid::new_v4();
    let paths = Paths::new(dir.path());
    paths.ensure_session(id).unwrap();
    let path = paths.catalog();
    let binding_store = AgentBindingStore::new(paths.agent_binding(id));
    let binding = binding(id, dir.path());
    std::fs::create_dir_all(
        std::path::Path::new(&binding.evidence_path)
            .parent()
            .unwrap(),
    )
    .unwrap();
    std::fs::write(
        &binding.evidence_path,
        format!(
            "{{\"type\":\"user\",\"sessionId\":\"{AGENT_SESSION_ID}\",\"cwd\":{}}}\n",
            serde_json::to_string(dir.path().to_str().unwrap()).unwrap()
        ),
    )
    .unwrap();
    {
        let mut catalog = Catalog::open(&path).unwrap();
        catalog.insert(&fixture(id)).unwrap();
        binding_store.save(&binding).unwrap();
        assert_eq!(catalog.bump_controller_generation().unwrap(), 1);
    }
    let mut catalog = Catalog::open(&path).unwrap();
    let restored = catalog.get(id).unwrap().unwrap();
    assert_eq!(restored.command, FALLBACK_SHELL);
    assert_eq!(restored.args, [LOGIN_SHELL_ARGUMENT]);
    let restored_binding = binding_store
        .load(&binding_context(id, dir.path()))
        .unwrap()
        .unwrap();
    assert_eq!(restored_binding.terminal_id, restored.terminal_id);
    assert_eq!(restored_binding.provider_session_id, AGENT_SESSION_ID);
    assert_eq!(catalog.bump_controller_generation().unwrap(), 2);
    catalog.integrity_check().unwrap();
}

#[test]
fn archived_sessions_are_hidden_but_not_deleted() {
    let dir = tempdir().unwrap();
    let mut catalog = Catalog::open(&dir.path().join("catalog.sqlite")).unwrap();
    let id = Uuid::new_v4();
    catalog.insert(&fixture(id)).unwrap();
    catalog.archive(id, true).unwrap();
    assert!(catalog.list(false).unwrap().is_empty());
    assert_eq!(catalog.list(true).unwrap().len(), 1);
    assert!(catalog.get(id).unwrap().is_some());
}
