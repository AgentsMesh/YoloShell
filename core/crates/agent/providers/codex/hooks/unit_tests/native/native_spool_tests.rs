use super::test_support::{CONFLICTING_FILE, Fixture, prompt, request};
use crate::codex_hooks::CodexNativeSpool;
use std::os::unix::fs::PermissionsExt;

#[test]
fn native_spool_replays_exact_occurrences_and_never_rebinds_an_id() {
    let fixture = Fixture::new();
    let callback = request(&fixture);
    let spool = CodexNativeSpool::for_scope(
        &fixture.paths,
        callback.terminal_id,
        callback.runtime_epoch,
    )
    .unwrap();
    let persisted = spool.persist(&callback).unwrap();
    assert_eq!(
        spool.read(callback.event_id).unwrap().as_ref(),
        Some(&persisted)
    );
    assert_eq!(spool.persist(&callback).unwrap(), persisted);
    let mut conflict = prompt(&fixture);
    conflict.event_id = callback.event_id;
    assert!(spool.persist(&conflict).is_err());
    spool.verify(&persisted).unwrap();
    spool.remove(&persisted).unwrap();
    assert!(spool.read(callback.event_id).unwrap().is_none());
    assert!(spool.remove(&persisted).is_err());
}

#[test]
fn native_spool_rejects_cross_runtime_publication() {
    let fixture = Fixture::new();
    let callback = request(&fixture);
    let spool = CodexNativeSpool::for_scope(
        &fixture.paths,
        callback.terminal_id,
        uuid::Uuid::new_v4(),
    )
    .unwrap();
    assert!(spool.persist(&callback).is_err());
    assert!(
        CodexNativeSpool::for_scope(&fixture.paths, uuid::Uuid::nil(), callback.runtime_epoch)
            .is_err()
    );
}

#[test]
fn identical_replacement_cannot_be_acknowledged_or_removed_by_stale_receipt() {
    let fixture = Fixture::new();
    let callback = request(&fixture);
    let spool = CodexNativeSpool::for_scope(
        &fixture.paths,
        callback.terminal_id,
        callback.runtime_epoch,
    )
    .unwrap();
    let persisted = spool.persist(&callback).unwrap();
    let path = fixture
        .paths
        .session_dir(callback.terminal_id)
        .join(super::constants::NATIVE_ARTIFACTS_DIRECTORY_NAME)
        .join(callback.runtime_epoch.to_string())
        .join(format!(
            "{}{}",
            callback.event_id,
            super::constants::NATIVE_REQUEST_SUFFIX
        ));
    let replacement = path.parent().unwrap().join(CONFLICTING_FILE);
    std::fs::write(&replacement, serde_json::to_vec(&callback).unwrap()).unwrap();
    std::fs::set_permissions(
        &replacement,
        std::fs::Permissions::from_mode(crate::paths::constants::PRIVATE_FILE_MODE),
    )
    .unwrap();
    std::fs::rename(&replacement, &path).unwrap();
    assert!(spool.verify(&persisted).is_err());
    assert!(spool.remove(&persisted).is_err());
    assert!(path.exists());
}

#[test]
fn durable_callbacks_survive_bridge_loss_and_reopen_without_invocation_artifacts() {
    let fixture = Fixture::new();
    let callback = prompt(&fixture);
    let persisted = CodexNativeSpool::for_scope(
        &fixture.paths,
        callback.terminal_id,
        callback.runtime_epoch,
    )
    .unwrap()
    .persist(&callback)
    .unwrap();
    let reopened = CodexNativeSpool::for_scope(
        &fixture.paths,
        callback.terminal_id,
        callback.runtime_epoch,
    )
    .unwrap();
    assert_eq!(
        reopened.read(callback.event_id).unwrap().as_ref(),
        Some(&persisted)
    );
    assert!(!fixture.artifacts.directory().exists());
}
