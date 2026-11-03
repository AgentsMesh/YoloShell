use crate::host::tests::*;
#[tokio::test]
async fn recovery_manifest_publication_waits_for_worker_manifest_fence_to_finish() {
    let (_directory, host, original) = recovery_manifest_fixture().await;
    let candidate = recovery_manifest_candidate(&original);
    let publication =
        RecoveryManifestTestPublication::capture(&host, &original, &candidate).unwrap();
    let runtime_path = host.paths.runtime_record(original.terminal_id);
    let mut runtime = host_runtime_record(
        &host,
        original.terminal_id,
        host.paths.worker_socket(original.terminal_id).unwrap(),
    );
    host.write_namespace_json(&runtime_path, &runtime).unwrap();
    let worker = crate::paths::durable_mutation::SessionDurableMutationGuard::acquire(
        &host.paths,
        &host.namespace,
        original.terminal_id,
    )
    .unwrap();
    let directory_identity = worker.session_identity();
    let (_, manifest_fence) = crate::paths::RetainedJsonFileObservation::bind::<TerminalManifest>(
        &host.paths.manifest(original.terminal_id),
    )
    .unwrap();
    std::thread::scope(|scope| {
        let host_writer = Arc::clone(&host);
        let writer = scope.spawn(move || publication.commit(&host_writer));
        let queued =
            wait_for_recovery_directory_waiter(directory_identity, || writer.is_finished());
        let unchanged = manifest_fence.verify_unchanged();
        let (_, runtime_identity): (RuntimeRecord, _) = worker
            .read_optional_with_identity(&runtime_path)
            .unwrap()
            .unwrap();
        runtime.controller_generation += RECOVERY_PUBLICATION_GENERATION_STEP;
        let worker_commit = worker.write_json_expected_with_fence(
            &runtime_path,
            Some(runtime_identity),
            &runtime,
            |_| manifest_fence.verify_unchanged(),
        );
        let final_fence = manifest_fence.verify_unchanged();
        drop(worker);
        let publication = writer.join().unwrap();
        queued.unwrap();
        unchanged.unwrap();
        worker_commit.unwrap();
        final_fence.unwrap();
        publication.unwrap();
    });
    let durable: TerminalManifest = read_json(&host.paths.manifest(original.terminal_id)).unwrap();
    assert_manifest_snapshot(&durable, &manifest_snapshot(&candidate));
    assert_eq!(read_json::<RuntimeRecord>(&runtime_path).unwrap(), runtime);
}

#[tokio::test]
async fn recovery_manifest_publication_does_not_reacquire_held_lifecycle() {
    let (_directory, host, original) = recovery_manifest_fixture().await;
    let candidate = recovery_manifest_candidate(&original);
    let lifecycle = host
        .paths
        .try_acquire_session_lifecycle_lock(original.terminal_id)
        .unwrap()
        .unwrap();

    host.persist_recovery_manifest(&original, &candidate)
        .unwrap();

    let durable: TerminalManifest = read_json(&host.paths.manifest(original.terminal_id)).unwrap();
    assert_manifest_snapshot(&durable, &manifest_snapshot(&candidate));
    drop(lifecycle);
}

#[tokio::test]
async fn recovery_manifest_cas_changes_only_recovery_state_and_timestamp() {
    let (_directory, host, original) = recovery_manifest_fixture().await;
    let candidate = recovery_manifest_candidate(&original);
    host.persist_recovery_manifest(&original, &candidate)
        .unwrap();
    let mut durable: TerminalManifest =
        read_json(&host.paths.manifest(original.terminal_id)).unwrap();
    assert_eq!(durable.recovery_state, candidate.recovery_state);
    assert_eq!(durable.updated_at_ms, candidate.updated_at_ms);
    durable.recovery_state = original.recovery_state;
    durable.updated_at_ms = original.updated_at_ms;
    assert_manifest_snapshot(&durable, &manifest_snapshot(&original));
}

#[tokio::test]
async fn required_unchanged_recovery_still_rejects_stale_metadata() {
    let (_directory, host, mut original) = recovery_manifest_fixture().await;
    let mut current = original.clone();
    current.name = RECOVERY_PUBLICATION_REPLACEMENT_NAME.into();
    host.write_namespace_json(&host.paths.manifest(original.terminal_id), &current)
        .unwrap();
    let before = manifest_snapshot(&original);
    let state = original.recovery_state;

    assert!(
        host.persist_recovery_state_required(&mut original, state)
            .await
            .is_err()
    );

    assert_manifest_snapshot(&original, &before);
    let durable: TerminalManifest = read_json(&host.paths.manifest(original.terminal_id)).unwrap();
    assert_manifest_snapshot(&durable, &manifest_snapshot(&current));
    let catalog = host
        .catalog_get(original.terminal_id)
        .await
        .unwrap()
        .unwrap();
    assert_manifest_snapshot(&catalog, &before);
}
