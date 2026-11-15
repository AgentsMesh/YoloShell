use super::*;
use std::sync::atomic::{AtomicBool, Ordering};

const ORIGINAL_VALUE: &str = "pending";
const REPLACEMENT_VALUE: &[u8] = b"replacement";
const INJECTED_PERSISTENCE_FAILURE: &str = "injected pending persistence failure";

#[test]
fn namespace_replacement_after_pending_write_prevents_go_progress() {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join("home");
    let displaced = root.path().join("displaced");
    let paths = Paths::new(&home);
    let terminal_id = Uuid::new_v4();
    paths.ensure_session(terminal_id).unwrap();
    let namespace = WorkerNamespace::capture(&paths, terminal_id).unwrap();
    let record = paths.runtime_record(terminal_id);
    let replacement = record.clone();
    let error = persist_bootstrap_record_with(&namespace, &record, &ORIGINAL_VALUE, || {
        std::fs::rename(&home, &displaced)?;
        std::fs::create_dir_all(replacement.parent().unwrap())?;
        std::fs::write(&replacement, REPLACEMENT_VALUE)?;
        Ok(())
    })
    .unwrap_err();
    assert!(error.to_string().contains("namespace"));
    assert_eq!(std::fs::read(replacement).unwrap(), REPLACEMENT_VALUE);
    let retained = displaced
        .join("sessions")
        .join(terminal_id.to_string())
        .join("runtime.json");
    let stored: String = read_json(&retained).unwrap();
    assert_eq!(stored, ORIGINAL_VALUE);
}

#[tokio::test]
async fn pending_persistence_must_complete_before_gate_release() {
    let persisted = AtomicBool::new(false);
    let released = AtomicBool::new(false);
    persist_pending_then_release(
        || {
            assert!(!released.load(Ordering::Acquire));
            persisted.store(true, Ordering::Release);
            Ok(())
        },
        || async {
            assert!(persisted.load(Ordering::Acquire));
            released.store(true, Ordering::Release);
            Ok(())
        },
    )
    .await
    .unwrap();
    assert!(released.load(Ordering::Acquire));
}

#[tokio::test]
async fn failed_pending_persistence_never_releases_gate() {
    let released = AtomicBool::new(false);
    let result = persist_pending_then_release(
        || anyhow::bail!(INJECTED_PERSISTENCE_FAILURE),
        || async {
            released.store(true, Ordering::Release);
            Ok(())
        },
    )
    .await;
    assert!(result.is_err());
    assert!(!released.load(Ordering::Acquire));
}
