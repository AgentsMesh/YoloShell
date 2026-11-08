use crate::host::tests::*;
use super::*;

const ARCHIVE_RACE_SCAN_DIAGNOSTICS: usize = 1;

#[tokio::test]
async fn durable_only_manifest_archives_without_a_catalog_row() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    host.paths.ensure_session(manifest.terminal_id).unwrap();
    host.write_namespace_json(&host.paths.manifest(manifest.terminal_id), &manifest)
        .unwrap();

    let archived = host
        .archive_bound_session(manifest.terminal_id)
        .await
        .unwrap();
    assert!(archived.manifest.archived);
    let durable: TerminalManifest = read_json(&host.paths.manifest(manifest.terminal_id)).unwrap();
    assert!(durable.archived);
}

#[tokio::test]
async fn catalog_only_row_cannot_stand_in_for_a_missing_manifest() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    host.catalog_insert(&manifest).await.unwrap();

    let error = host
        .archive_bound_session(manifest.terminal_id)
        .await
        .unwrap_err();
    assert_eq!(error.code, crate::protocol::error_code::NOT_FOUND);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stale_catalog_repair_cannot_reverse_an_archive_commit() {
    let directory = tempdir().unwrap();
    let host = Arc::new(Host::open(HostConfig::new(directory.path())).unwrap());
    let indexed = fixture_manifest(directory.path());
    seed_recovery_manifest(&host, &indexed).await;

    let mut stale_durable = indexed.clone();
    stale_durable.recovery_state = RecoveryState::Uncertain;
    stale_durable.updated_at_ms = now_ms();
    host.persist_recovery_manifest(&indexed, &stale_durable)
        .unwrap();

    let (reached, repair_reached) = std::sync::mpsc::channel();
    let (release, repair_release) = std::sync::mpsc::channel();
    let (completed, repair_completed) = std::sync::mpsc::channel();
    *lock_recover(&host.catalog_test_probes.scan) = Some(ReconcileScanProbe::repair(
        reached,
        repair_release,
        completed,
    ));
    let scan_host = Arc::clone(&host);
    let scan = tokio::spawn(async move { scan_host.reconciliation_manifests().await });
    tokio::task::spawn_blocking(move || {
        repair_reached
            .recv_timeout(HOST_BLOCKING_TEST_TIMEOUT)
            .unwrap()
    })
    .await
    .unwrap();

    let archived = host
        .archive_bound_session(indexed.terminal_id)
        .await
        .unwrap();
    assert!(archived.manifest.archived);
    release.send(()).unwrap();
    let (recovered, diagnostics) = scan.await.unwrap();
    let repair_outcome = repair_completed
        .recv_timeout(HOST_BLOCKING_TEST_TIMEOUT)
        .unwrap();

    assert_eq!(repair_outcome, CatalogRepairProbeOutcome::Superseded);
    assert!(recovered.is_empty());
    assert_eq!(diagnostics, ARCHIVE_RACE_SCAN_DIAGNOSTICS);
    let catalog = host
        .catalog_get(indexed.terminal_id)
        .await
        .unwrap()
        .unwrap();
    let durable: TerminalManifest = read_json(&host.paths.manifest(indexed.terminal_id)).unwrap();
    assert!(catalog.archived);
    assert!(durable.archived);
    assert_eq!(durable.recovery_state, RecoveryState::Uncertain);
    assert_eq!(catalog.recovery_state, RecoveryState::Uncertain);
    assert!(
        host.archive_bound_session(indexed.terminal_id)
            .await
            .unwrap()
            .manifest
            .archived
    );
}
