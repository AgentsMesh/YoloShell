use super::*;
use crate::agent::constants::{BINDING_SCHEMA_VERSION, DATA_HOME_FIELD};
use crate::paths::atomic_write_json;
use serde_json::json;
use std::fs;
use std::path::Path;
use tempfile::tempdir;
use uuid::Uuid;

const CODEX_SESSION_ID: &str = "550e8400-e29b-41d4-a716-446655440000";
const REPLACEMENT_SESSION_ID: &str = "7d444840-9dc0-11d1-b245-5ffdce74fad2";
const EVIDENCE_RELATIVE_PATH: &str =
    "sessions/2026/08/05/rollout-2026-08-05T12-00-00-550e8400-e29b-41d4-a716-446655440000.jsonl";
const OTHER_CWD_DIRECTORY: &str = "other-cwd";
const OTHER_HOME_DIRECTORY: &str = "other-home";

fn context(directory: &Path, session_id: Uuid) -> DetectionContext {
    DetectionContext::new(
        session_id,
        directory.into(),
        [
            (AgentKind::Codex, directory.join("codex-home")),
            (AgentKind::ClaudeCode, directory.join("claude-home")),
        ],
    )
}

fn binding(directory: &Path, session_id: Uuid) -> AgentBinding {
    let data_home = directory.join("codex-home");
    AgentBinding {
        schema_version: BINDING_SCHEMA_VERSION,
        terminal_id: session_id,
        kind: AgentKind::Codex,
        provider_session_id: CODEX_SESSION_ID.into(),
        executable: crate::agent::codex::constants::EXECUTABLE.into(),
        cwd: directory.display().to_string(),
        evidence_path: data_home.join(EVIDENCE_RELATIVE_PATH).display().to_string(),
        data_home: data_home.display().to_string(),
        process_pid: Some(17),
        process_start_time: Some(17),
        state: AgentState::Unknown,
        recovery_availability: crate::agent::AgentRecoveryAvailability::UnmanagedLaunch,
        detected_at_ms: 10,
        updated_at_ms: 20,
    }
}

fn write_evidence(binding: &AgentBinding) {
    let path = Path::new(&binding.evidence_path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let record = json!({
        (crate::agent::codex::constants::RECORD_TYPE_KEY): crate::agent::codex::constants::SESSION_META_TYPE,
        (crate::agent::codex::constants::PAYLOAD_KEY): {
            (crate::agent::codex::constants::SESSION_ID_KEY): binding.provider_session_id,
            (crate::agent::codex::constants::CWD_KEY): binding.cwd,
            (crate::agent::codex::constants::SOURCE_KEY): crate::agent::codex::constants::CLI_SOURCE,
        },
    });
    fs::write(path, format!("{record}{}", constants::COMMAND_LINE_ENDING)).unwrap();
}

fn assert_load_error(store: &AgentBindingStore, context: &DetectionContext, expected: &str) {
    let error = store.load(context).unwrap_err().to_string();
    assert!(
        error.contains(expected),
        "expected {expected:?} in {error:?}"
    );
}

#[test]
fn missing_data_home_is_rejected_and_store_path_is_exposed() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("binding.json");
    let session_id = Uuid::new_v4();
    let mut value = serde_json::to_value(binding(directory.path(), session_id)).unwrap();
    value.as_object_mut().unwrap().remove(DATA_HOME_FIELD);
    atomic_write_json(&path, &value).unwrap();

    let store = AgentBindingStore::new(&path);
    assert_eq!(store.path(), path);
    let error = store
        .load(&context(directory.path(), session_id))
        .unwrap_err();
    assert!(error.to_string().contains("decode agent binding"));
}

#[test]
fn load_distinguishes_decode_and_filesystem_failures() {
    let directory = tempdir().unwrap();
    let context = context(directory.path(), Uuid::new_v4());
    let malformed = directory.path().join("malformed.json");
    atomic_write_json(&malformed, &"not an agent binding").unwrap();
    let error = AgentBindingStore::new(malformed)
        .load(&context)
        .unwrap_err();
    assert!(error.to_string().contains("decode agent binding"));

    let error = AgentBindingStore::new(directory.path())
        .load(&context)
        .unwrap_err();
    assert!(error.to_string().contains("read agent binding"));
}

#[test]
fn save_reports_an_unusable_parent_path() {
    let directory = tempdir().unwrap();
    let parent = directory.path().join("ordinary-file");
    fs::write(&parent, b"occupied").unwrap();
    let store = AgentBindingStore::new(parent.join("binding.json"));
    let error = store
        .save(&binding(directory.path(), Uuid::new_v4()))
        .unwrap_err();
    assert!(error.to_string().contains("persist agent binding"));
}

#[test]
fn load_fences_identity_and_evidence_while_preserving_verified_codex_cwd() {
    let directory = tempdir().unwrap();
    let root = directory.path().canonicalize().unwrap();
    let session_id = Uuid::new_v4();
    let context = context(&root, session_id);
    let path = root.join("binding.json");
    let store = AgentBindingStore::new(&path);
    let valid = binding(&root, session_id);
    write_evidence(&valid);

    let mut wrong_schema = valid.clone();
    wrong_schema.schema_version += 1;
    store.save(&wrong_schema).unwrap();
    assert_load_error(&store, &context, constants::validation::SCHEMA);

    let mut cross_session = valid.clone();
    cross_session.terminal_id = Uuid::new_v4();
    store.save(&cross_session).unwrap();
    assert_load_error(&store, &context, constants::validation::YOLO_SESSION);

    let mut wrong_executable = valid.clone();
    wrong_executable.executable = "sh".into();
    store.save(&wrong_executable).unwrap();
    assert_load_error(&store, &context, constants::validation::EXECUTABLE);

    let mut wrong_provider = valid.clone();
    wrong_provider.kind = AgentKind::ClaudeCode;
    wrong_provider.executable = crate::agent::claude::constants::EXECUTABLE.into();
    store.save(&wrong_provider).unwrap();
    assert_load_error(&store, &context, constants::validation::EVIDENCE_PATH);

    let other_cwd = root.join(OTHER_CWD_DIRECTORY);
    let mut invocation_cwd = valid.clone();
    invocation_cwd.cwd = other_cwd.display().to_string();
    store.save(&invocation_cwd).unwrap();
    assert_load_error(&store, &context, constants::validation::CWD);
    fs::create_dir(&other_cwd).unwrap();
    assert_eq!(store.load(&context).unwrap(), Some(invocation_cwd));

    let other_home = root.join(OTHER_HOME_DIRECTORY);
    fs::create_dir(&other_home).unwrap();
    let mut wrong_home = valid.clone();
    wrong_home.data_home = other_home.display().to_string();
    store.save(&wrong_home).unwrap();
    assert_load_error(&store, &context, constants::validation::EVIDENCE_PATH);

    let mut exited = valid.clone();
    exited.state = AgentState::Exited;
    exited.recovery_availability = crate::agent::AgentRecoveryAvailability::Unavailable;
    exited.process_pid = None;
    exited.process_start_time = None;
    store.save(&exited).unwrap();
    assert_eq!(store.load(&context).unwrap(), Some(exited));

    store.save(&valid).unwrap();
    let mut replaced = valid.clone();
    replaced.provider_session_id = REPLACEMENT_SESSION_ID.into();
    write_evidence(&replaced);
    assert_load_error(&store, &context, constants::validation::STALE_EVIDENCE);
}
