use crate::host::tests::*;
#[tokio::test]
async fn archive_lock_excludes_runtime_publication_after_the_final_validation() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    seed_recovery_manifest(&host, &manifest).await;
    let selection = absent_runtime_selection(&manifest);
    let replacement = host_runtime_record(
        &host,
        manifest.terminal_id,
        directory.path().join(ARCHIVE_BOUNDARY_SOCKET_NAME),
    );
    let attempted = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let published = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let attempted_by_hook = Arc::clone(&attempted);
    let published_by_hook = Arc::clone(&published);
    let paths = host.paths.clone();
    *lock_recover(&host.catalog_test_probes.archive_publication) = Some(Box::new(move || {
        attempted_by_hook.store(true, std::sync::atomic::Ordering::SeqCst);
        if let Some(_lock) = paths
            .try_acquire_session_lifecycle_lock(replacement.terminal_id)
            .unwrap()
        {
            atomic_write_json(&paths.runtime_record(replacement.terminal_id), &replacement).unwrap();
            published_by_hook.store(true, std::sync::atomic::Ordering::SeqCst);
        }
    }));

    let archived = host.archive_product_session(selection).await.unwrap();

    assert!(archived.manifest.archived);
    assert!(attempted.load(std::sync::atomic::Ordering::SeqCst));
    assert!(!published.load(std::sync::atomic::Ordering::SeqCst));
    assert!(!host.paths.runtime_record(manifest.terminal_id).exists());
    assert!(
        read_json::<TerminalManifest>(&host.paths.manifest(manifest.terminal_id))
            .unwrap()
            .archived
    );
}

#[tokio::test]
async fn idempotent_archive_rejects_potentially_live_runtime_custody() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let mut manifest = fixture_manifest(directory.path());
    manifest.archived = true;
    seed_recovery_manifest(&host, &manifest).await;
    let runtime = host_runtime_record(
        &host,
        manifest.terminal_id,
        directory.path().join(ARCHIVED_RUNTIME_SOCKET_NAME),
    );
    atomic_write_json(&host.paths.runtime_record(manifest.terminal_id), &runtime).unwrap();

    let error = host
        .archive_bound_session(manifest.terminal_id)
        .await
        .unwrap_err();

    assert_eq!(error.code, crate::protocol::error_code::CONFLICT);
    assert_eq!(error.message, ARCHIVED_RUNTIME_CONFLICT);
    assert_eq!(
        read_json::<RuntimeRecord>(&host.paths.runtime_record(manifest.terminal_id)).unwrap(),
        runtime
    );
}

#[tokio::test]
async fn internal_archive_rejects_an_active_startup_checkpoint() {
    let fixture = CreationCheckpointFixture::new(false).await;
    fixture.write(&fixture.active_checkpoint());

    let error = fixture
        .host
        .archive_bound_session(fixture.binding.result.terminal_id)
        .await
        .unwrap_err();

    assert_eq!(error.code, crate::protocol::error_code::CONFLICT);
    assert!(!fixture.manifest().archived);
}

const ARCHIVE_BOUNDARY_SOCKET_NAME: &str = "archive-boundary-runtime.sock";
const ARCHIVED_RUNTIME_SOCKET_NAME: &str = "archived-live-runtime.sock";
