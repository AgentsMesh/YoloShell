use crate::host::tests::*;
const IDEMPOTENT_RECOVERY_TIMESTAMP: i64 = 1;
const CHANGED_RECOVERY_NAME: &str = "updated recovery session";

#[tokio::test]
async fn same_recovery_state_cannot_publish_uncommitted_manifest_metadata() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let mut manifest = fixture_manifest(directory.path());
    manifest.created_at_ms = IDEMPOTENT_RECOVERY_TIMESTAMP;
    manifest.updated_at_ms = IDEMPOTENT_RECOVERY_TIMESTAMP;
    manifest.recovery_state = RecoveryState::Uncertain;
    seed_recovery_manifest(&host, &manifest).await;
    let expected = manifest_snapshot(&manifest);
    manifest.name = CHANGED_RECOVERY_NAME.into();
    let caller_before = manifest_snapshot(&manifest);

    host.persist_recovery_state(&mut manifest, RecoveryState::Uncertain)
        .await;

    assert_manifest_snapshot(&manifest, &caller_before);
    let durable: TerminalManifest = read_json(&host.paths.manifest(manifest.terminal_id)).unwrap();
    assert_manifest_snapshot(&durable, &expected);
    assert_catalog_and_wire_snapshot(&host, manifest.terminal_id, &expected).await;
}

#[tokio::test]
async fn unchanged_automatic_recovery_block_keeps_selection_and_durable_identity() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let mut manifest = fixture_manifest(directory.path());
    manifest.created_at_ms = IDEMPOTENT_RECOVERY_TIMESTAMP;
    manifest.updated_at_ms = IDEMPOTENT_RECOVERY_TIMESTAMP;
    manifest.recovery_state = RecoveryState::Uncertain;
    seed_recovery_manifest(&host, &manifest).await;
    let expected = manifest_snapshot(&manifest);
    let (_, observation) = crate::paths::RetainedJsonFileObservation::bind::<TerminalManifest>(
        &host.paths.manifest(manifest.terminal_id),
    )
    .unwrap();

    host.converge_automatic_resume_block(manifest.terminal_id)
        .await
        .unwrap();

    observation.verify_unchanged().unwrap();
    assert_catalog_and_wire_snapshot(&host, manifest.terminal_id, &expected).await;
}

#[tokio::test]
async fn unchanged_recovery_publication_keeps_selection_and_durable_identity() {
    for state in [RecoveryState::NeedsRecreate, RecoveryState::Uncertain] {
        let directory = tempdir().unwrap();
        let host = Host::open(HostConfig::new(directory.path())).unwrap();
        let mut manifest = fixture_manifest(directory.path());
        manifest.created_at_ms = IDEMPOTENT_RECOVERY_TIMESTAMP;
        manifest.updated_at_ms = IDEMPOTENT_RECOVERY_TIMESTAMP;
        manifest.recovery_state = state;
        seed_recovery_manifest(&host, &manifest).await;
        let expected = manifest_snapshot(&manifest);
        let (_, observation) = crate::paths::RetainedJsonFileObservation::bind::<TerminalManifest>(
            &host.paths.manifest(manifest.terminal_id),
        )
        .unwrap();

        host.persist_recovery_state(&mut manifest, state).await;
        host.persist_recovery_state_durable(&mut manifest, state);

        assert_manifest_snapshot(&manifest, &expected);
        observation.verify_unchanged().unwrap();
        assert_catalog_and_wire_snapshot(&host, manifest.terminal_id, &expected).await;
    }
}

#[tokio::test]
async fn unchanged_recovery_publication_retries_a_failed_catalog_write() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let mut manifest = fixture_manifest(directory.path());
    seed_recovery_manifest(&host, &manifest).await;
    *lock_recover(&host.catalog_test_probes.upsert_failure) = true;
    host.persist_recovery_state(&mut manifest, RecoveryState::NeedsRecreate)
        .await;
    let expected = manifest_snapshot(&manifest);
    let (_, observation) = crate::paths::RetainedJsonFileObservation::bind::<TerminalManifest>(
        &host.paths.manifest(manifest.terminal_id),
    )
    .unwrap();
    *lock_recover(&host.catalog_test_probes.upsert_failure) = false;

    host.persist_recovery_state(&mut manifest, RecoveryState::NeedsRecreate)
        .await;

    observation.verify_unchanged().unwrap();
    assert_manifest_snapshot(&manifest, &expected);
    assert_catalog_and_wire_snapshot(&host, manifest.terminal_id, &expected).await;
}
