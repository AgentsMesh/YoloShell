use crate::host::tests::*;
#[tokio::test]
async fn recovery_manifest_cas_rejects_archive_and_same_clock_metadata_replacement() {
    for archive in [false, true] {
        let (_directory, host, original) = recovery_manifest_fixture().await;
        let candidate = recovery_manifest_candidate(&original);
        let publication =
            RecoveryManifestTestPublication::capture(&host, &original, &candidate).unwrap();
        let mut replacement = original.clone();
        if archive {
            replacement.archived = true;
        } else {
            replacement.name = RECOVERY_PUBLICATION_REPLACEMENT_NAME.into();
        }
        let path = host.paths.manifest(original.terminal_id);
        host.write_namespace_json(&path, &replacement).unwrap();
        let (_, observation) =
            crate::paths::RetainedJsonFileObservation::bind::<TerminalManifest>(&path).unwrap();

        assert!(publication.commit(&host).is_err());

        observation.verify_unchanged().unwrap();
        assert_manifest_snapshot(
            &read_json::<TerminalManifest>(&path).unwrap(),
            &manifest_snapshot(&replacement),
        );
    }
}

#[tokio::test]
async fn recovery_manifest_cas_rejects_same_bytes_aba() {
    let (_directory, host, original) = recovery_manifest_fixture().await;
    let candidate = recovery_manifest_candidate(&original);
    let publication =
        RecoveryManifestTestPublication::capture(&host, &original, &candidate).unwrap();
    let mut intermediate = original.clone();
    intermediate.name = RECOVERY_PUBLICATION_REPLACEMENT_NAME.into();
    let path = host.paths.manifest(original.terminal_id);
    host.write_namespace_json(&path, &intermediate).unwrap();
    host.write_namespace_json(&path, &original).unwrap();
    let (_, observation) =
        crate::paths::RetainedJsonFileObservation::bind::<TerminalManifest>(&path).unwrap();

    assert!(publication.commit(&host).is_err());

    observation.verify_unchanged().unwrap();
}

#[tokio::test]
async fn recovery_manifest_observed_capture_rejects_aba_before_preparation() {
    let (_directory, host, original) = recovery_manifest_fixture().await;
    let path = host.paths.manifest(original.terminal_id);
    let (_, identity): (TerminalManifest, _) =
        host.namespace.read_json_with_identity(&path).unwrap();
    host.write_namespace_json(&path, &original).unwrap();
    let (_, observation) =
        crate::paths::RetainedJsonFileObservation::bind::<TerminalManifest>(&path).unwrap();
    let candidate = recovery_manifest_candidate(&original);
    assert!(
        RecoveryManifestTestPublication::capture_observed(&host, &original, identity, &candidate)
            .is_err()
    );
    observation.verify_unchanged().unwrap();
}

#[tokio::test]
async fn recovery_manifest_capture_rejects_non_recovery_candidate_fields() {
    let mutations: &[fn(&mut TerminalManifest)] = &[
        |manifest| manifest.terminal_id = Uuid::new_v4(),
        |manifest| manifest.schema_version += 1,
        |manifest| manifest.archived = true,
        |manifest| manifest.name = RECOVERY_PUBLICATION_REPLACEMENT_NAME.into(),
        |manifest| {
            manifest
                .command
                .push_str(RECOVERY_PUBLICATION_REPLACEMENT_NAME)
        },
        |manifest| {
            manifest
                .args
                .push(RECOVERY_PUBLICATION_REPLACEMENT_NAME.into())
        },
        |manifest| manifest.cwd.push(RECOVERY_PUBLICATION_REPLACEMENT_NAME),
        |manifest| manifest.created_at_ms += RECOVERY_PUBLICATION_TIMESTAMP_STEP,
        |manifest| manifest.cols += 1,
        |manifest| manifest.rows += 1,
    ];
    let (_directory, host, original) = recovery_manifest_fixture().await;
    let (_, observation) = crate::paths::RetainedJsonFileObservation::bind::<TerminalManifest>(
        &host.paths.manifest(original.terminal_id),
    )
    .unwrap();
    for mutate in mutations {
        let mut candidate = recovery_manifest_candidate(&original);
        mutate(&mut candidate);
        assert!(RecoveryManifestTestPublication::capture(&host, &original, &candidate).is_err());
    }
    observation.verify_unchanged().unwrap();
}

#[tokio::test]
async fn recovery_manifest_cas_does_not_recreate_a_removed_manifest() {
    let (_directory, host, original) = recovery_manifest_fixture().await;
    let candidate = recovery_manifest_candidate(&original);
    let publication =
        RecoveryManifestTestPublication::capture(&host, &original, &candidate).unwrap();
    let path = host.paths.manifest(original.terminal_id);
    host.namespace.remove_file(&path).unwrap();

    assert!(publication.commit(&host).is_err());

    assert!(!path.exists());
}

#[tokio::test]
async fn recovery_manifest_cas_rejects_a_pending_session_mutation() {
    let (_directory, host, original) = recovery_manifest_fixture().await;
    let candidate = recovery_manifest_candidate(&original);
    let publication =
        RecoveryManifestTestPublication::capture(&host, &original, &candidate).unwrap();
    host.write_namespace_json(
        &host.paths.reboot_migration_intent(original.terminal_id),
        &serde_json::Value::Null,
    )
    .unwrap();

    assert!(publication.commit(&host).is_err());

    let durable: TerminalManifest = read_json(&host.paths.manifest(original.terminal_id)).unwrap();
    assert_manifest_snapshot(&durable, &manifest_snapshot(&original));
}

#[tokio::test]
async fn recovery_manifest_cas_rejects_rebound_session_directory() {
    let (directory, host, original) = recovery_manifest_fixture().await;
    let candidate = recovery_manifest_candidate(&original);
    let publication =
        RecoveryManifestTestPublication::capture(&host, &original, &candidate).unwrap();
    let session = host.paths.session_dir(original.terminal_id);
    std::fs::rename(
        &session,
        directory
            .path()
            .join(RECOVERY_PUBLICATION_REPLACEMENT_DIRECTORY),
    )
    .unwrap();
    crate::paths::directory::PrivateDirectory::ensure(&session).unwrap();
    atomic_write_json(&host.paths.manifest(original.terminal_id), &original).unwrap();

    assert!(publication.commit(&host).is_err());

    let durable: TerminalManifest = read_json(&host.paths.manifest(original.terminal_id)).unwrap();
    assert_manifest_snapshot(&durable, &manifest_snapshot(&original));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn recovery_catalog_publication_rejects_archive_and_aba_after_durable_cas() {
    for archive in [false, true] {
        let (_directory, host, original) = recovery_manifest_fixture().await;
        let pause = pause_manifest_publication(&host);
        let task_host = Arc::clone(&host);
        let mut caller = original.clone();
        let task = tokio::spawn(async move {
            let result = task_host
                .persist_recovery_state_required(&mut caller, RecoveryState::Uncertain)
                .await;
            (result, caller)
        });
        let release = pause.wait().await;
        let path = host.paths.manifest(original.terminal_id);
        let mut replacement: TerminalManifest = read_json(&path).unwrap();
        replacement.archived = archive;
        host.write_namespace_json(&path, &replacement).unwrap();
        release.send(()).unwrap();
        let (result, caller) = task.await.unwrap();

        assert!(result.is_err());
        assert_manifest_snapshot(&caller, &manifest_snapshot(&original));
        let catalog = host
            .catalog_get(original.terminal_id)
            .await
            .unwrap()
            .unwrap();
        assert_manifest_snapshot(&catalog, &manifest_snapshot(&original));
        let durable: TerminalManifest = read_json(&path).unwrap();
        assert_manifest_snapshot(&durable, &manifest_snapshot(&replacement));
    }
}
