use crate::host::tests::*;
const CREATION_COMMIT_DEADLINE_SESSION_NAME: &str = "commit-deadline-session";

pub(super) fn creation_commit_deadline() -> HostOperationDeadline {
    HostOperationDeadline::expiring_at_commit(HOST_BLOCKING_TEST_TIMEOUT)
}

#[tokio::test]
async fn creation_deadline_at_directory_commit_cannot_create_session_namespace() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let (_, binding, _) = reserve_test_creation(
        &host,
        directory.path(),
        CREATION_COMMIT_DEADLINE_SESSION_NAME,
    );

    let error = host
        .prepare_bound_session_before(&binding, creation_commit_deadline())
        .await
        .unwrap_err();

    assert_eq!(error.code, crate::protocol::error_code::DELIVERY_UNKNOWN);
    assert!(!host.paths.session_dir(binding.result.terminal_id).exists());
    assert!(
        host.catalog_get(binding.result.terminal_id)
            .await
            .unwrap()
            .is_none()
    );
}

#[test]
fn creation_deadline_at_manifest_commit_preserves_missing_manifest() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    host.ensure_session_namespace(manifest.terminal_id).unwrap();
    let path = host.paths.manifest(manifest.terminal_id);

    let error = host
        .write_creation_json_before(&path, &manifest, creation_commit_deadline())
        .unwrap_err();

    assert_eq!(error.code, crate::protocol::error_code::DELIVERY_UNKNOWN);
    assert!(host.namespace.file_is_missing(&path).unwrap());
}

#[tokio::test]
async fn creation_deadline_at_catalog_commit_rolls_back_insert() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());

    let error = host
        .catalog_insert_before(&manifest, creation_commit_deadline())
        .await
        .unwrap_err();

    assert!(error.is::<CreationPublicationDeadlineExpired>());
    assert!(
        host.catalog_get(manifest.terminal_id)
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn creation_deadline_at_catalog_upsert_commit_preserves_previous_manifest() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    host.catalog_insert(&manifest).await.unwrap();
    let mut replacement = manifest.clone();
    replacement.recovery_state = RecoveryState::Uncertain;

    let error = host
        .catalog_upsert_before(&replacement, creation_commit_deadline())
        .await
        .unwrap_err();

    assert!(error.is::<CreationPublicationDeadlineExpired>());
    let stored = host
        .catalog_get(manifest.terminal_id)
        .await
        .unwrap()
        .unwrap();
    assert!(same_manifest(&stored, &manifest));
}

#[tokio::test]
async fn creation_uncertain_publication_is_fenced_before_manifest_commit() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let mut manifest = fixture_manifest(directory.path());
    host.ensure_session_namespace(manifest.terminal_id).unwrap();
    host.write_namespace_json(&host.paths.manifest(manifest.terminal_id), &manifest)
        .unwrap();
    host.catalog_insert(&manifest).await.unwrap();

    let error = host
        .mark_creation_worker_start_uncertain_before(
            &mut manifest,
            &anyhow::anyhow!(WORKER_START_PUBLIC_DETAIL),
            creation_commit_deadline(),
        )
        .await
        .unwrap_err();

    assert_eq!(error.code, crate::protocol::error_code::DELIVERY_UNKNOWN);
    assert_eq!(
        host.load_durable_manifest(manifest.terminal_id)
            .unwrap()
            .recovery_state,
        RecoveryState::Created
    );
    assert_eq!(
        host.catalog_get(manifest.terminal_id)
            .await
            .unwrap()
            .unwrap()
            .recovery_state,
        RecoveryState::Created
    );
}

#[tokio::test]
async fn creation_retry_repairs_a_partial_uncertain_manifest_publication() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let (_, binding, _) = reserve_test_creation(
        &host,
        directory.path(),
        CREATION_COMMIT_DEADLINE_SESSION_NAME,
    );
    let manifest = host.prepare_bound_session(&binding).await.unwrap();
    let mut durable = manifest.clone();
    durable.recovery_state = RecoveryState::Uncertain;
    host.write_namespace_json(&host.paths.manifest(manifest.terminal_id), &durable)
        .unwrap();

    let repaired = host
        .existing_bound_manifest_before(
            &binding,
            HostOperationDeadline::after(HOST_BLOCKING_TEST_TIMEOUT),
        )
        .await
        .unwrap()
        .unwrap();

    assert!(same_manifest(&repaired, &durable));
    let catalog = host
        .catalog_get(manifest.terminal_id)
        .await
        .unwrap()
        .unwrap();
    assert!(same_manifest(&catalog, &durable));
}

#[test]
fn creation_deadline_at_worker_launch_commit_preserves_missing_launch() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    host.ensure_session_namespace(manifest.terminal_id).unwrap();
    let launch = shell_resume_launch(&host, &manifest);
    let path = host.paths.worker_launch(manifest.terminal_id);

    let error = host
        .publish_creation_worker_launch_before(&launch, creation_commit_deadline())
        .unwrap_err();

    assert!(error.is::<CreationPublicationDeadlineExpired>());
    assert!(host.namespace.file_is_missing(&path).unwrap());
}

#[test]
fn creation_deadline_at_receipt_commit_preserves_empty_indexes() {
    let (_directory, store, binding, _) = receipt_test_setup();
    let receipt = CreationReceipt::from_binding(&binding, CreationCompletion::Finalized);
    let name = binding.creation_token_digest.as_str();

    let error = publish_receipt_at_before(
        &store.staging,
        &store.token_index,
        name,
        &receipt,
        creation_commit_deadline(),
    )
    .unwrap_err();

    assert!(error.is::<CreationPublicationDeadlineExpired>());
    assert_receipt_indexes_empty(&store);
}
