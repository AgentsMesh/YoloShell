use crate::host::tests::*;
#[tokio::test]
async fn explicit_recreate_requires_an_exact_catalog_selection() {
    let (_directory, host, manifest, _runtime, store) = recreate_fence_fixture();
    let error = host
        .resume_session(ResumeRequest {
            selection: TerminalLifecycleSelection::unconditional(manifest.terminal_id),
            recreate: true,
        })
        .await
        .unwrap_err();

    assert_eq!(error.code, crate::protocol::error_code::CONFLICT);
    assert_recreate_fence_retained(&host, &store, manifest.terminal_id);
}

#[tokio::test]
async fn explicit_recreate_rejects_a_stale_catalog_status() {
    let (_directory, host, manifest, runtime, store) = recreate_fence_fixture();
    let resume_request = exact_recreate_request(&manifest, &runtime, TerminalRuntimeStatus::Lost);
    persist_archive_exit(&host, &runtime, runtime.runtime_epoch);

    let response = host
        .dispatch(&request(method::RESUME_SESSION_EXACT, &resume_request))
        .await;
    let error = response.error.unwrap();

    assert_eq!(error.code, crate::protocol::error_code::CONFLICT);
    assert_recreate_fence_retained(&host, &store, manifest.terminal_id);
}

#[tokio::test]
async fn explicit_recreate_rejects_a_stale_manifest_digest() {
    let (_directory, host, manifest, runtime, store) = recreate_fence_fixture();
    let request = exact_recreate_request(&manifest, &runtime, TerminalRuntimeStatus::Lost);
    let mut changed = manifest.clone();
    changed.name.push_str(RESUME_RECREATE_CHANGED_NAME_SUFFIX);
    changed.updated_at_ms = changed
        .updated_at_ms
        .saturating_add(RESUME_RECREATE_UPDATED_AT_INCREMENT);
    atomic_write_json(&host.paths.manifest(changed.terminal_id), &changed).unwrap();
    host.catalog_upsert(&changed).await.unwrap();

    let error = host.resume_session(request).await.unwrap_err();

    assert_eq!(error.code, crate::protocol::error_code::CONFLICT);
    assert_recreate_fence_retained(&host, &store, manifest.terminal_id);
}

#[tokio::test]
async fn explicit_recreate_rejects_a_stale_runtime_epoch() {
    let (_directory, host, manifest, runtime, store) = recreate_fence_fixture();
    let request = exact_recreate_request(&manifest, &runtime, TerminalRuntimeStatus::Lost);
    let mut changed = runtime;
    changed.runtime_epoch = Uuid::new_v4();
    changed.worker_id = Uuid::new_v4();
    host.write_namespace_json(&host.paths.runtime_record(manifest.terminal_id), &changed)
        .unwrap();

    let error = host.resume_session(request).await.unwrap_err();

    assert_eq!(error.code, crate::protocol::error_code::CONFLICT);
    assert_recreate_fence_retained(&host, &store, manifest.terminal_id);
}

#[tokio::test]
async fn explicit_recreate_revalidates_runtime_under_the_publication_lock() {
    let (_directory, host, manifest, runtime, store) = recreate_fence_fixture();
    let request = exact_recreate_request(&manifest, &runtime, TerminalRuntimeStatus::Lost);
    let runtime_path = host.paths.runtime_record(manifest.terminal_id);
    let mut replacement = runtime;
    replacement.runtime_epoch = Uuid::new_v4();
    replacement.worker_id = Uuid::new_v4();
    let expected = replacement.clone();
    *lock_recover(&host.catalog_test_probes.lifecycle_revalidation) = Some(Box::new(move || {
        atomic_write_json(&runtime_path, &replacement).unwrap();
    }));

    let error = host.resume_session(request).await.unwrap_err();

    assert_eq!(error.code, crate::protocol::error_code::CONFLICT);
    assert!(lock_recover(&host.catalog_test_probes.lifecycle_revalidation).is_none());
    assert_recreate_fence_retained(&host, &store, manifest.terminal_id);
    assert_eq!(
        read_json::<RuntimeRecord>(&host.paths.runtime_record(manifest.terminal_id)).unwrap(),
        expected
    );
}

fn recreate_fence_fixture() -> (
    tempfile::TempDir,
    Host,
    TerminalManifest,
    RuntimeRecord,
    AgentBindingStore,
) {
    let directory = tempdir().unwrap();
    let host = unspawnable_resume_host(&directory);
    let manifest = fixture_manifest(directory.path());
    let runtime = persist_dead_resume_session(&host, &manifest);
    let store = AgentBindingStore::new(host.paths.agent_binding(manifest.terminal_id));
    store.save(&host_binding(manifest.terminal_id)).unwrap();
    (directory, host, manifest, runtime, store)
}

fn assert_recreate_fence_retained(host: &Host, store: &AgentBindingStore, terminal_id: Uuid) {
    assert!(store.path().exists());
    assert!(!host.paths.resume_transaction(terminal_id).exists());
    assert!(!host.paths.resume_binding_clear(terminal_id).exists());
}

const RESUME_RECREATE_CHANGED_NAME_SUFFIX: &str = "-changed";
const RESUME_RECREATE_UPDATED_AT_INCREMENT: i64 = 1;
