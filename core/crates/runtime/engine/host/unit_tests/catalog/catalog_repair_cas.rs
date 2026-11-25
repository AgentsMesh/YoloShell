use crate::host::tests::*;
use super::*;
const SUPERSEDED_SCAN_DIAGNOSTICS: usize = 1;
const NONCANONICAL_SCAN_DIAGNOSTICS: usize = 1;
const RECOVERED_SESSION_COUNT: usize = 1;
const EMPTY_ALIAS_COUNT: i64 = 0;
const NONCANONICAL_SESSION_ID_TEXT: &str = "abcdef01-2345-4678-9abc-def012345678";
const RENAME_CATALOG_ROW_SQL: &str = "UPDATE sessions SET session_id = ?1 WHERE session_id = ?2";
const COUNT_CATALOG_ROW_SQL: &str = "SELECT COUNT(*) FROM sessions WHERE session_id = ?1";
const REPLACEMENT_HOME_NAME: &str = "replacement-home";
const RETAINED_HOME_NAME: &str = "retained-home";
const DISPLACED_HOME_NAME: &str = "displaced-home";

#[tokio::test]
async fn running_scan_repairs_a_noncanonical_valid_catalog_key() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let mut manifest = fixture_manifest(directory.path());
    manifest.terminal_id = Uuid::parse_str(NONCANONICAL_SESSION_ID_TEXT).unwrap();
    seed_recovery_manifest(&host, &manifest).await;
    let expected = manifest_snapshot(&manifest);
    let canonical = manifest.terminal_id.to_string();
    let alias = canonical.to_uppercase();
    assert_ne!(alias, canonical);
    rusqlite::Connection::open(host.paths.catalog())
        .unwrap()
        .execute(RENAME_CATALOG_ROW_SQL, rusqlite::params![alias, canonical])
        .unwrap();

    let (recovered, diagnostics) = host.reconciliation_manifests().await;

    assert_eq!(diagnostics, NONCANONICAL_SCAN_DIAGNOSTICS);
    assert_eq!(recovered.len(), RECOVERED_SESSION_COUNT);
    assert_manifest_snapshot(&recovered[0], &expected);
    assert_catalog_and_wire_snapshot(&host, manifest.terminal_id, &expected).await;
    let alias_count: i64 = rusqlite::Connection::open(host.paths.catalog())
        .unwrap()
        .query_row(COUNT_CATALOG_ROW_SQL, [alias], |row| row.get(0))
        .unwrap();
    assert_eq!(alias_count, EMPTY_ALIAS_COUNT);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn same_generation_repair_cannot_replace_a_newer_session_commit() {
    let directory = tempdir().unwrap();
    let host = Arc::new(Host::open(HostConfig::new(directory.path())).unwrap());
    let original = fixture_manifest(directory.path());
    seed_recovery_manifest(&host, &original).await;
    let mut first = original.clone();
    first.recovery_state = RecoveryState::Uncertain;
    first.updated_at_ms = now_ms();
    host.persist_recovery_manifest(&original, &first).unwrap();
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

    let mut current = first;
    host.persist_recovery_state(&mut current, RecoveryState::NeedsRecreate)
        .await;
    let expected = manifest_snapshot(&current);
    release.send(()).unwrap();
    let (recovered, diagnostics) = scan.await.unwrap();
    let outcome = repair_completed
        .recv_timeout(HOST_BLOCKING_TEST_TIMEOUT)
        .unwrap();

    assert_eq!(outcome, CatalogRepairProbeOutcome::Superseded);
    assert!(recovered.is_empty());
    assert_eq!(diagnostics, SUPERSEDED_SCAN_DIAGNOSTICS);
    let durable: TerminalManifest = read_json(&host.paths.manifest(current.terminal_id)).unwrap();
    assert_manifest_snapshot(&durable, &expected);
    assert_catalog_and_wire_snapshot(&host, current.terminal_id, &expected).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ancestor_replacement_cannot_redirect_a_verified_catalog_repair() {
    use std::os::unix::fs::PermissionsExt;

    let parent = tempdir().unwrap();
    let home = parent.path().join(RETAINED_HOME_NAME);
    std::fs::create_dir(&home).unwrap();
    std::fs::set_permissions(
        &home,
        std::fs::Permissions::from_mode(crate::host::TEST_PRIVATE_DIRECTORY_MODE),
    )
    .unwrap();
    let host = Arc::new(Host::open(HostConfig::new(&home)).unwrap());
    let indexed = fixture_manifest(&home);
    seed_recovery_manifest(&host, &indexed).await;
    let mut durable = indexed.clone();
    durable.recovery_state = RecoveryState::Uncertain;
    durable.updated_at_ms = now_ms();
    host.persist_recovery_manifest(&indexed, &durable).unwrap();
    let replacement_home = parent.path().join(REPLACEMENT_HOME_NAME);
    std::fs::create_dir(&replacement_home).unwrap();
    std::fs::set_permissions(
        &replacement_home,
        std::fs::Permissions::from_mode(crate::host::TEST_PRIVATE_DIRECTORY_MODE),
    )
    .unwrap();
    let replacement_paths = Paths::new(&replacement_home);
    let mut replacement = Catalog::open(&replacement_paths.catalog()).unwrap();
    replacement.insert(&indexed).unwrap();
    assert_eq!(
        replacement.bump_controller_generation().unwrap(),
        host.controller_generation
    );
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
    repair_reached
        .recv_timeout(HOST_BLOCKING_TEST_TIMEOUT)
        .unwrap();

    let displaced_home = parent.path().join(DISPLACED_HOME_NAME);
    std::fs::rename(&home, displaced_home).unwrap();
    std::fs::rename(replacement_home, &home).unwrap();
    release.send(()).unwrap();

    let (recovered, diagnostics) = scan.await.unwrap();
    let outcome = repair_completed
        .recv_timeout(HOST_BLOCKING_TEST_TIMEOUT)
        .unwrap();
    assert_eq!(outcome, CatalogRepairProbeOutcome::Applied);
    assert!(recovered.is_empty());
    assert_eq!(diagnostics, SUPERSEDED_SCAN_DIAGNOSTICS);
    assert_eq!(
        manifest_snapshot(&replacement.get(indexed.terminal_id).unwrap().unwrap()),
        manifest_snapshot(&indexed)
    );
    assert_eq!(
        replacement.controller_generation().unwrap(),
        host.controller_generation
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn newer_retained_catalog_generation_fences_a_paused_repair() {
    let directory = tempdir().unwrap();
    let host = Arc::new(Host::open(HostConfig::new(directory.path())).unwrap());
    let indexed = fixture_manifest(directory.path());
    seed_recovery_manifest(&host, &indexed).await;
    let mut durable = indexed.clone();
    durable.recovery_state = RecoveryState::NeedsRecreate;
    durable.updated_at_ms = now_ms();
    host.persist_recovery_manifest(&indexed, &durable).unwrap();
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
    repair_reached
        .recv_timeout(HOST_BLOCKING_TEST_TIMEOUT)
        .unwrap();
    let next_generation = lock_recover(&host.catalog)
        .bump_controller_generation()
        .unwrap();
    assert!(next_generation > host.controller_generation);
    release.send(()).unwrap();

    let (recovered, diagnostics) = scan.await.unwrap();
    let outcome = repair_completed
        .recv_timeout(HOST_BLOCKING_TEST_TIMEOUT)
        .unwrap();
    assert_eq!(outcome, CatalogRepairProbeOutcome::Fenced);
    assert!(recovered.is_empty());
    assert_eq!(diagnostics, SUPERSEDED_SCAN_DIAGNOSTICS);
    let stored = lock_recover(&host.catalog)
        .get(indexed.terminal_id)
        .unwrap()
        .unwrap();
    assert_eq!(manifest_snapshot(&stored), manifest_snapshot(&indexed));
}
