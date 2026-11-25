use crate::host::tests::*;
const RECOVERY_CATALOG_PROVIDER_FAILURE: &str = "provider requires explicit recreation";

fn recovery_catalog_terminal_failure() -> SessionResumeFailure {
    SessionResumeFailure::TerminalRecreate(ErrorBody::new(
        crate::protocol::error_code::RECREATE_REQUIRED,
        RECOVERY_CATALOG_PROVIDER_FAILURE,
        false,
    ))
}

#[tokio::test]
async fn recovery_catalog_failure_rejects_a_foreign_request_without_dumping_its_manifest() {
    let (_directory, host, original) = recovery_manifest_fixture().await;
    let identity = resume_manifest_identity(&host, original.terminal_id);
    let context =
        recovery_manifest_publication::RecoveryCatalogFailure::new(original.clone(), identity);

    assert!(context.downgrade(&host, Uuid::new_v4()).is_err());

    assert_eq!(
        resume_manifest_identity(&host, original.terminal_id),
        identity
    );
    let diagnostic = format!("{context:?}");
    assert_eq!(diagnostic, context.to_string());
    assert!(!diagnostic.contains(&original.terminal_id.to_string()));
}

#[tokio::test]
async fn recovery_catalog_failure_downgrades_its_exact_durable_commit() {
    let (_directory, host, original) = recovery_manifest_fixture().await;
    *lock_recover(&host.catalog_test_probes.upsert_failure) = true;
    let mut caller = original.clone();

    let failure = host
        .persist_classified_resume_failure(
            &mut caller,
            RecoveryState::NeedsRecreate,
            recovery_catalog_terminal_failure(),
        )
        .await;

    assert!(matches!(failure, SessionResumeFailure::Protocol(_)));
    let durable: TerminalManifest = read_json(&host.paths.manifest(original.terminal_id)).unwrap();
    assert_eq!(durable.recovery_state, RecoveryState::Uncertain);
    assert_manifest_snapshot(&caller, &manifest_snapshot(&durable));
    let mut expected = original;
    expected.recovery_state = RecoveryState::Uncertain;
    expected.updated_at_ms = durable.updated_at_ms;
    assert_manifest_snapshot(&durable, &manifest_snapshot(&expected));
}

#[derive(Clone, Copy)]
enum RecoveryCatalogReplacement {
    SameBytes,
    Archive,
    Metadata,
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn recovery_catalog_failure_never_downgrades_a_replacement_after_its_commit() {
    for mutation in [
        RecoveryCatalogReplacement::SameBytes,
        RecoveryCatalogReplacement::Archive,
        RecoveryCatalogReplacement::Metadata,
    ] {
        let directory = tempdir().unwrap();
        let host = Arc::new(Host::open(HostConfig::new(directory.path())).unwrap());
        let mut original = fixture_manifest(directory.path());
        original.recovery_state = RecoveryState::NeedsRecreate;
        seed_recovery_manifest(&host, &original).await;
        let pause = pause_manifest_publication(&host);
        let task_host = Arc::clone(&host);
        let mut caller = original.clone();
        let task = tokio::spawn(async move {
            let failure = task_host
                .persist_classified_resume_failure(
                    &mut caller,
                    RecoveryState::NeedsRecreate,
                    recovery_catalog_terminal_failure(),
                )
                .await;
            (failure, caller)
        });
        let release = pause.wait().await;
        let mut replacement = original.clone();
        match mutation {
            RecoveryCatalogReplacement::SameBytes => {}
            RecoveryCatalogReplacement::Archive => replacement.archived = true,
            RecoveryCatalogReplacement::Metadata => {
                replacement.name = RECOVERY_PUBLICATION_REPLACEMENT_NAME.into();
            }
        }
        let path = host.paths.manifest(original.terminal_id);
        host.write_namespace_json(&path, &replacement).unwrap();
        let replacement_identity = resume_manifest_identity(&host, original.terminal_id);
        release.send(()).unwrap();
        let (failure, caller) = task.await.unwrap();

        assert!(matches!(failure, SessionResumeFailure::Protocol(_)));
        assert_manifest_snapshot(&caller, &manifest_snapshot(&original));
        assert_eq!(
            resume_manifest_identity(&host, original.terminal_id),
            replacement_identity
        );
        let durable: TerminalManifest = read_json(&path).unwrap();
        assert_manifest_snapshot(&durable, &manifest_snapshot(&replacement));
        let catalog = host
            .catalog_get(original.terminal_id)
            .await
            .unwrap()
            .unwrap();
        assert_manifest_snapshot(&catalog, &manifest_snapshot(&original));
    }
}
