use super::*;
use crate::agent::claude::generation::{
    AuthorityProcessIdentity, AuthorityRuntimeIdentity, ClaudeGenerationStore,
};
use crate::agent::constants::BINDING_SCHEMA_VERSION;
use crate::agent::hooks::ProcessIdentity;
use crate::agent::{AgentDetector, AgentKind, AgentState, DetectionResult};
use crate::namespace_lease::NamespaceLease;
use crate::paths::atomic_write_json;
use serde_json::json;
use tempfile::tempdir;
use uuid::Uuid;

const CODEX_SESSION_ID: &str = "550e8400-e29b-41d4-a716-446655440000";
const EVIDENCE_DATE_DIRECTORY: &str = "2026/08/05";
const EVIDENCE_FILE: &str =
    "rollout-2026-08-05T12-00-00-550e8400-e29b-41d4-a716-446655440000.jsonl";
const NAMESPACE_SCOPE: &str = "binding recovery";
const PROVIDER_PID: u32 = 1;
const PROVIDER_START_TIME: u64 = 1;
const WORKER_PID: u32 = 701;
const WORKER_START_TIME: u64 = 702;
const UNRELATED_CLAUDE_AUTHORITY: &[u8] = b"unrelated-claude-authority";

#[test]
fn binding_round_trip_is_atomic() {
    let directory = tempdir().unwrap();
    let root = directory.path().canonicalize().unwrap();
    let path = root.join("binding.json");
    crate::paths::ensure_private_parent(&path).unwrap();
    let store = AgentBindingStore::new(path);
    let lease = NamespaceLease::capture(&[(NAMESPACE_SCOPE, root.as_path())]).unwrap();
    let session_id = Uuid::new_v4();
    let data_home = root.join("codex-home");
    let evidence_root = data_home.join(crate::agent::codex::constants::EVIDENCE_DIR);
    std::fs::create_dir_all(&evidence_root).unwrap();
    let evidence = evidence_root
        .join(EVIDENCE_DATE_DIRECTORY)
        .join(EVIDENCE_FILE);
    std::fs::create_dir_all(evidence.parent().unwrap()).unwrap();
    let record = json!({
        (crate::agent::codex::constants::RECORD_TYPE_KEY): crate::agent::codex::constants::SESSION_META_TYPE,
        (crate::agent::codex::constants::PAYLOAD_KEY): {
            (crate::agent::codex::constants::SESSION_ID_KEY): CODEX_SESSION_ID,
            (crate::agent::codex::constants::CWD_KEY): root,
            (crate::agent::codex::constants::SOURCE_KEY): crate::agent::codex::constants::CLI_SOURCE,
        },
    });
    std::fs::write(
        &evidence,
        format!("{record}{}", crate::agent::constants::COMMAND_LINE_ENDING),
    )
    .unwrap();
    let context = DetectionContext::new(
        session_id,
        root.clone(),
        [(AgentKind::Codex, data_home.clone())],
    );
    assert_eq!(store.load(&context).unwrap(), None);
    let binding = AgentBinding {
        schema_version: BINDING_SCHEMA_VERSION,
        terminal_id: session_id,
        kind: AgentKind::Codex,
        provider_session_id: CODEX_SESSION_ID.into(),
        executable: crate::agent::codex::constants::EXECUTABLE.into(),
        cwd: root.display().to_string(),
        evidence_path: evidence.display().to_string(),
        data_home: data_home.display().to_string(),
        process_pid: Some(PROVIDER_PID),
        process_start_time: Some(PROVIDER_START_TIME),
        state: AgentState::Unknown,
        recovery_availability: crate::agent::AgentRecoveryAvailability::UnmanagedLaunch,
        detected_at_ms: 1,
        updated_at_ms: 1,
    };
    store.save(&binding).unwrap();
    assert_eq!(store.load(&context).unwrap(), Some(binding.clone()));
    let generation = ClaudeGenerationStore::from_binding_path(store.path()).unwrap();
    std::fs::write(generation.path(), UNRELATED_CLAUDE_AUTHORITY).unwrap();
    let runtime = runtime();
    let mut managed = binding.clone();
    managed.state = AgentState::Running;
    managed.recovery_availability = AgentRecoveryAvailability::Available;
    codex_authority_test_support::publish(
        &store,
        &lease,
        runtime,
        ProcessIdentity::new(PROVIDER_PID, PROVIDER_START_TIME),
        &managed,
    )
    .unwrap();
    assert_eq!(
        store
            .load_published_authoritative_in_namespace(&lease, &context, runtime)
            .unwrap(),
        Some(managed.clone())
    );
    std::fs::remove_file(generation.path()).unwrap();

    let mut raw_unknown = managed;
    raw_unknown.state = AgentState::Unknown;
    raw_unknown.recovery_availability = AgentRecoveryAvailability::Available;
    crate::paths::atomic_write_json(store.path(), &raw_unknown).unwrap();
    let mut unknown = raw_unknown.clone();
    unknown.recovery_availability = AgentRecoveryAvailability::Unavailable;
    assert_eq!(store.load(&context).unwrap(), Some(unknown.clone()));
    assert_eq!(
        store
            .load_authoritative_in_namespace(&lease, &context, runtime)
            .unwrap(),
        Some(unknown.clone())
    );

    let mut detector =
        AgentDetector::new(context.clone(), yolo_agent_providers::builtin_agent_registry(), Some(raw_unknown)).unwrap();
    assert_eq!(detector.session_exited(), DetectionResult::Unchanged);
    store.save(&unknown).unwrap();
    assert_eq!(store.load(&context).unwrap(), Some(unknown));
}

#[path = "../test_support/codex_authority.rs"]
mod codex_authority_test_support;

fn runtime() -> AuthorityRuntimeIdentity {
    AuthorityRuntimeIdentity {
        runtime_epoch: Uuid::new_v4(),
        worker: AuthorityProcessIdentity {
            pid: WORKER_PID,
            start_time: WORKER_START_TIME,
        },
    }
}

#[test]
fn clear_is_idempotent_but_preserves_non_missing_io_errors() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("binding.json");
    crate::paths::ensure_private_parent(&path).unwrap();
    let store = AgentBindingStore::new(&path);
    store.clear().unwrap();
    AgentBindingStore::new(directory.path().join("missing/binding.json"))
        .clear()
        .unwrap();
    crate::paths::atomic_write_json(&path, &"binding").unwrap();
    store.clear().unwrap();
    assert!(!path.exists());

    let error = AgentBindingStore::new(directory.path())
        .clear()
        .unwrap_err();
    assert!(error.to_string().contains("remove agent binding"));
}

#[cfg(unix)]
#[test]
fn load_rejects_a_binding_symlink() {
    let directory = tempdir().unwrap();
    let target = directory.path().join("target.json");
    let binding = directory.path().join("binding.json");
    atomic_write_json(&target, &serde_json::json!({})).unwrap();
    std::os::unix::fs::symlink(&target, &binding).unwrap();
    let context = DetectionContext::new(
        Uuid::new_v4(),
        directory.path().into(),
        [(AgentKind::Codex, directory.path().join("codex-home"))],
    );
    assert!(AgentBindingStore::new(binding).load(&context).is_err());
}

#[cfg(unix)]
#[test]
fn clear_rejects_symlink_and_hardlink_entries_without_following() {
    let directory = tempdir().unwrap();
    let source = directory.path().join("source.json");
    crate::paths::atomic_write_json(&source, &serde_json::json!({"kept": true})).unwrap();
    let original = std::fs::read(&source).unwrap();

    let symbolic = directory.path().join("symbolic.json");
    std::os::unix::fs::symlink(&source, &symbolic).unwrap();
    assert!(AgentBindingStore::new(&symbolic).clear().is_err());
    assert_eq!(std::fs::read(&source).unwrap(), original);
    assert!(
        std::fs::symlink_metadata(&symbolic)
            .unwrap()
            .file_type()
            .is_symlink()
    );

    let linked = directory.path().join("linked.json");
    std::fs::hard_link(&source, &linked).unwrap();
    assert!(AgentBindingStore::new(&linked).clear().is_err());
    assert_eq!(std::fs::read(&source).unwrap(), original);
    assert!(linked.exists());
}
