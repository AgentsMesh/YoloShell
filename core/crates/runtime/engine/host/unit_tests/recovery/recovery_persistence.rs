use crate::host::tests::*;
use super::*;

#[tokio::test]
async fn durable_recovery_state_precedes_catalog_publication() {
    let directory = tempdir().unwrap();
    let host = Arc::new(Host::open(HostConfig::new(directory.path())).unwrap());
    let mut manifest = fixture_manifest(directory.path());
    let terminal_id = manifest.terminal_id;
    seed_recovery_manifest(&host, &manifest).await;
    let (operations, mut observed) = tokio::sync::mpsc::unbounded_channel();
    *lock_recover(&host.catalog_test_probes.operations) = Some(operations);
    let task_host = Arc::clone(&host);
    let task = tokio::spawn(async move {
        task_host
            .persist_recovery_state(&mut manifest, RecoveryState::NeedsRecreate)
            .await;
        manifest
    });

    observed.recv().await.unwrap();

    let durable: TerminalManifest = read_json(&host.paths.manifest(terminal_id)).unwrap();
    assert_eq!(durable.recovery_state, RecoveryState::NeedsRecreate);
    let caller = task.await.unwrap();
    assert_manifest_snapshot(&caller, &manifest_snapshot(&durable));
    let catalog = host.catalog_get(terminal_id).await.unwrap().unwrap();
    assert_manifest_snapshot(&catalog, &manifest_snapshot(&durable));
}

#[tokio::test]
async fn failed_durable_recovery_write_is_not_published_to_catalog() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let mut manifest = fixture_manifest(directory.path());
    seed_recovery_manifest(&host, &manifest).await;
    let expected = manifest_snapshot(&manifest);
    block_recovery_manifest_write(&host, manifest.terminal_id);
    let (operations, mut observed) = tokio::sync::mpsc::unbounded_channel();
    *lock_recover(&host.catalog_test_probes.operations) = Some(operations);

    host.persist_recovery_state(&mut manifest, RecoveryState::NeedsRecreate)
        .await;

    assert_manifest_snapshot(&manifest, &expected);
    assert!(matches!(
        observed.try_recv(),
        Err(tokio::sync::mpsc::error::TryRecvError::Empty)
    ));
    *lock_recover(&host.catalog_test_probes.operations) = None;
    assert_catalog_and_wire_snapshot(&host, manifest.terminal_id, &expected).await;
}

#[tokio::test]
async fn failed_durable_only_recovery_write_does_not_mutate_caller() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let mut manifest = fixture_manifest(directory.path());
    seed_recovery_manifest(&host, &manifest).await;
    let expected = manifest_snapshot(&manifest);
    block_recovery_manifest_write(&host, manifest.terminal_id);

    host.persist_recovery_state_durable(&mut manifest, RecoveryState::Uncertain);

    assert_manifest_snapshot(&manifest, &expected);
    assert_catalog_and_wire_snapshot(&host, manifest.terminal_id, &expected).await;
}

#[tokio::test]
async fn failed_recovery_write_is_retained_and_retried_after_storage_recovers() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let mut manifest = fixture_manifest(directory.path());
    let terminal_id = manifest.terminal_id;
    seed_recovery_manifest(&host, &manifest).await;
    block_recovery_manifest_write(&host, terminal_id);

    assert!(!host.persist_recovery_state_durable(&mut manifest, RecoveryState::Uncertain));
    assert_eq!(lock_recover(&host.pending_recovery_writes).len(), 1);
    assert_eq!(host.retry_pending_recovery_writes().await, 1);
    assert_eq!(lock_recover(&host.pending_recovery_writes).len(), 1);

    std::fs::remove_dir(host.paths.manifest(terminal_id)).unwrap();
    host.write_namespace_json(&host.paths.manifest(terminal_id), &manifest)
        .unwrap();
    assert_eq!(host.retry_pending_recovery_writes().await, 0);
    assert!(lock_recover(&host.pending_recovery_writes).is_empty());
    let durable: TerminalManifest = read_json(&host.paths.manifest(terminal_id)).unwrap();
    assert_eq!(durable.recovery_state, RecoveryState::Uncertain);
}

#[tokio::test]
async fn pending_recovery_write_yields_to_newer_durable_state_or_archive() {
    for (newer_state, archived) in [
        (RecoveryState::Reattached, false),
        (RecoveryState::Created, true),
    ] {
        let directory = tempdir().unwrap();
        let host = Host::open(HostConfig::new(directory.path())).unwrap();
        let mut manifest = fixture_manifest(directory.path());
        let terminal_id = manifest.terminal_id;
        seed_recovery_manifest(&host, &manifest).await;
        block_recovery_manifest_write(&host, terminal_id);
        assert!(!host.persist_recovery_state_durable(&mut manifest, RecoveryState::Uncertain));
        assert_eq!(lock_recover(&host.pending_recovery_writes).len(), 1);

        std::fs::remove_dir(host.paths.manifest(terminal_id)).unwrap();
        let mut newer = manifest.clone();
        newer.recovery_state = newer_state;
        newer.archived = archived;
        host.write_namespace_json(&host.paths.manifest(terminal_id), &newer)
            .unwrap();

        assert_eq!(host.retry_pending_recovery_writes().await, 0);
        assert!(lock_recover(&host.pending_recovery_writes).is_empty());
        let durable: TerminalManifest = read_json(&host.paths.manifest(terminal_id)).unwrap();
        assert_eq!(durable.recovery_state, newer_state);
        assert_eq!(durable.archived, archived);
    }
}

#[tokio::test]
async fn pending_recovery_write_yields_to_a_newer_generation_with_the_same_state() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let mut manifest = fixture_manifest(directory.path());
    let terminal_id = manifest.terminal_id;
    seed_recovery_manifest(&host, &manifest).await;
    block_recovery_manifest_write(&host, terminal_id);
    assert!(!host.persist_recovery_state_durable(&mut manifest, RecoveryState::Uncertain));
    assert_eq!(lock_recover(&host.pending_recovery_writes).len(), 1);

    std::fs::remove_dir(host.paths.manifest(terminal_id)).unwrap();
    let mut newer = manifest.clone();
    newer.name = format!("{}-newer", newer.name);
    newer.updated_at_ms = newer.updated_at_ms.saturating_add(1);
    host.write_namespace_json(&host.paths.manifest(terminal_id), &newer)
        .unwrap();

    assert_eq!(host.retry_pending_recovery_writes().await, 0);
    assert!(lock_recover(&host.pending_recovery_writes).is_empty());
    let durable: TerminalManifest = read_json(&host.paths.manifest(terminal_id)).unwrap();
    assert_eq!(durable.name, newer.name);
    assert_eq!(durable.recovery_state, RecoveryState::Created);
}

#[tokio::test]
async fn same_host_scan_repairs_a_failed_catalog_publication_from_durable_state() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let mut manifest = fixture_manifest(directory.path());
    seed_recovery_manifest(&host, &manifest).await;
    let original = manifest_snapshot(&manifest);
    *lock_recover(&host.catalog_test_probes.upsert_failure) = true;

    host.persist_recovery_state(&mut manifest, RecoveryState::NeedsRecreate)
        .await;

    let durable: TerminalManifest = read_json(&host.paths.manifest(manifest.terminal_id)).unwrap();
    let committed = manifest_snapshot(&durable);
    assert_manifest_snapshot(&manifest, &committed);
    assert_catalog_and_wire_snapshot(&host, manifest.terminal_id, &original).await;
    let (mut recovered, diagnostics) = host.reconciliation_manifests().await;
    assert_eq!(diagnostics, usize::default());
    assert_manifest_snapshot(&recovered.pop().unwrap(), &committed);
    assert!(recovered.is_empty());
    assert_catalog_and_wire_snapshot(&host, manifest.terminal_id, &committed).await;
}

include!("recovery_state_idempotence.rs");
include!("../../test_support/unit/recovery_manifest_publication_support.rs");
include!("recovery_manifest_publication_tests.rs");
include!("recovery_manifest_publication_adversarial_tests.rs");
include!("recovery_catalog_rollback_tests.rs");

pub(super) fn block_recovery_manifest_write(host: &Host, terminal_id: Uuid) {
    let manifest_path = host.paths.manifest(terminal_id);
    std::fs::remove_file(&manifest_path).unwrap();
    std::fs::create_dir(&manifest_path).unwrap();
}


include!("recovery_snapshot_assertions.rs");
