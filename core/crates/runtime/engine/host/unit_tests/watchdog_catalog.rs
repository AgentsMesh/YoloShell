use crate::host::tests::*;
use super::*;

const CORRUPT_CATALOG_ROW_SQL: &str =
    "UPDATE sessions SET manifest_json = ?1 WHERE session_id = ?2";

#[tokio::test]
async fn corrupt_catalog_session_is_reported_repaired_and_isolated() {
    let directory = tempdir().unwrap();
    let mut config = HostConfig::new(directory.path());
    config.runtime_executable = Some(directory.path().join("missing-runtime"));
    let host = Host::open(config).unwrap();
    let corrupt = fixture_manifest(directory.path());
    host.paths.ensure_session(corrupt.terminal_id).unwrap();
    host.catalog.lock().unwrap().insert(&corrupt).unwrap();
    atomic_write_json(&host.paths.manifest(corrupt.terminal_id), &corrupt).unwrap();
    let recoverable = fixture_manifest(directory.path());
    let mut record = host_runtime_record(
        &host,
        recoverable.terminal_id,
        directory.path().join("dead.sock"),
    );
    install_reaped_runtime_processes(&mut record);
    persist_host_session(&host, &recoverable, &record);
    host.secondary_custody
        .install_test_retirement_evidence(&host, &record);
    corrupt_catalog_manifest(&host, corrupt.terminal_id);
    let result = host.reconcile().await;
    assert_eq!(result.needs_recreate, 1);
    let durable = host.load_durable_manifest(recoverable.terminal_id).unwrap();
    assert_eq!(durable.recovery_state, RecoveryState::NeedsRecreate);
    assert_eq!(durable.terminal_id, recoverable.terminal_id);
    let retained: RuntimeRecord =
        read_json(&host.paths.runtime_record(recoverable.terminal_id)).unwrap();
    assert_eq!(retained, record);
    let exit: ExitRecord = read_json(&host.paths.exit_record(recoverable.terminal_id)).unwrap();
    assert_eq!(exit.reason, ExitReason::WorkerCrash);
    assert_eq!(exit.runtime_epoch, record.runtime_epoch);
    assert!(!host.paths.worker_launch(recoverable.terminal_id).exists());
    assert!(
        !host
            .paths
            .resume_transaction(recoverable.terminal_id)
            .exists()
    );
    assert!(result.unreachable >= 2);
    assert_eq!(host.catalog.lock().unwrap().list(false).unwrap().len(), 2);
}

#[tokio::test]
async fn mismatched_durable_manifest_cannot_overwrite_a_catalog_row() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let catalog_manifest = fixture_manifest(directory.path());
    host.paths
        .ensure_session(catalog_manifest.terminal_id)
        .unwrap();
    host.catalog
        .lock()
        .unwrap()
        .insert(&catalog_manifest)
        .unwrap();
    let mut wrong = catalog_manifest.clone();
    wrong.terminal_id = Uuid::new_v4();
    atomic_write_json(&host.paths.manifest(catalog_manifest.terminal_id), &wrong).unwrap();
    corrupt_catalog_manifest(&host, catalog_manifest.terminal_id);
    let schema_manifest = fixture_manifest(directory.path());
    host.paths
        .ensure_session(schema_manifest.terminal_id)
        .unwrap();
    host.catalog
        .lock()
        .unwrap()
        .insert(&schema_manifest)
        .unwrap();
    let mut unsupported = schema_manifest.clone();
    unsupported.schema_version = SESSION_MANIFEST_SCHEMA_VERSION + 1;
    atomic_write_json(
        &host.paths.manifest(schema_manifest.terminal_id),
        &unsupported,
    )
    .unwrap();
    corrupt_catalog_manifest(&host, schema_manifest.terminal_id);
    let result = host.reconcile().await;
    assert_eq!(result.unreachable, 2);
    assert!(host.catalog.lock().unwrap().list(false).is_err());
}

pub(super) fn corrupt_catalog_manifest(host: &Host, terminal_id: Uuid) {
    rusqlite::Connection::open(host.paths.catalog())
        .unwrap()
        .execute(
            CORRUPT_CATALOG_ROW_SQL,
            rusqlite::params![b"invalid-json", terminal_id.to_string()],
        )
        .unwrap();
}

#[tokio::test]
async fn stale_reconcile_snapshot_cannot_retry_needs_recreate() {
    let directory = tempdir().unwrap();
    let mut config = HostConfig::new(directory.path());
    config.runtime_executable = Some(directory.path().join("missing-runtime"));
    let host = Host::open(config).unwrap();
    let stale = fixture_manifest(directory.path());
    let mut record = host_runtime_record(
        &host,
        stale.terminal_id,
        directory.path().join("dead-worker.sock"),
    );
    record.worker_pid = u32::MAX;
    record.worker_start_time = 1;
    record.child_pid = u32::MAX;
    record.child_start_time = 1;
    persist_host_session(&host, &stale, &record);
    let mut current = stale.clone();
    current.recovery_state = RecoveryState::NeedsRecreate;
    host.catalog.lock().unwrap().upsert(&current).unwrap();
    atomic_write_json(&host.paths.manifest(current.terminal_id), &current).unwrap();
    let result = host.reconcile_manifest(stale).await;
    assert_eq!(result.unreachable, 1);
    assert_eq!(result.needs_recreate, 0);
    assert!(!host.paths.worker_launch(current.terminal_id).exists());
}

#[tokio::test]
async fn stale_reconcile_snapshot_cannot_restore_deleted_or_archived_sessions() {
    let directory = tempdir().unwrap();
    let mut config = HostConfig::new(directory.path());
    config.runtime_executable = Some(directory.path().join("missing-runtime"));
    let host = Host::open(config).unwrap();
    let archived = persisted_dead_session(&host, directory.path(), "archived.sock");
    host.catalog
        .lock()
        .unwrap()
        .archive(archived.terminal_id, true)
        .unwrap();
    let deleted = persisted_dead_session(&host, directory.path(), "deleted.sock");
    host.catalog
        .lock()
        .unwrap()
        .delete(deleted.terminal_id)
        .unwrap();
    assert_eq!(
        host.reconcile_manifest(archived.clone()).await.unreachable,
        0
    );
    assert_eq!(
        host.reconcile_manifest(deleted.clone()).await.unreachable,
        0
    );
    assert!(!host.paths.worker_launch(archived.terminal_id).exists());
    assert!(!host.paths.worker_launch(deleted.terminal_id).exists());
}

fn persisted_dead_session(host: &Host, directory: &Path, socket: &str) -> TerminalManifest {
    let manifest = fixture_manifest(directory);
    let mut record = host_runtime_record(host, manifest.terminal_id, directory.join(socket));
    record.worker_pid = u32::MAX;
    record.worker_start_time = 1;
    record.child_pid = u32::MAX;
    record.child_start_time = 1;
    persist_host_session(host, &manifest, &record);
    manifest
}

#[tokio::test]
async fn one_session_reconciliation_panic_does_not_abort_the_sweep() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    for name in ["first", "second"] {
        let mut manifest = fixture_manifest(directory.path());
        manifest.name = name.into();
        host.paths.ensure_session(manifest.terminal_id).unwrap();
        host.catalog.lock().unwrap().insert(&manifest).unwrap();
        atomic_write_json(&host.paths.manifest(manifest.terminal_id), &manifest).unwrap();
    }
    let poisoned = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _entries = host.session_locks.entries.lock().unwrap();
        panic!("injected session-lock registry panic");
    }));
    assert!(poisoned.is_err());
    let result = host.reconcile().await;
    assert_eq!(result.unreachable, 2);
}
