use crate::host::tests::*;
use super::*;

#[tokio::test]
async fn manifest_digest_rejects_same_timestamp_and_status_aba() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let original = fixture_manifest(directory.path());
    seed_recovery_manifest(&host, &original).await;
    let original_digest = crate::model::TerminalManifestDigest::capture(&original).unwrap();
    let stale = TerminalLifecycleSelection::unconditional(original.terminal_id)
        .with_expected_manifest_sha256(Some(original_digest.clone()));
    host.validate_product_selection(&stale).await.unwrap();

    let mut replacement = original.clone();
    replacement.name = SAME_CLOCK_REPLACEMENT_NAME.into();
    assert_eq!(replacement.updated_at_ms, original.updated_at_ms);
    assert!(!replacement.archived);
    host.catalog_upsert(&replacement).await.unwrap();

    let boundary_error = host.revalidate_lifecycle_fence(&stale).await.unwrap_err();
    assert_eq!(boundary_error.code, crate::protocol::error_code::CONFLICT);
    assert_eq!(
        host.validate_product_selection(&stale)
            .await
            .unwrap_err()
            .code,
        crate::protocol::error_code::CONFLICT
    );
    let archive_error = host
        .archive_product_session(stale.clone())
        .await
        .unwrap_err();
    let discard_error = host
        .discard_product_session(TerminalDiscardRequest {
            selection: stale,
            force: true,
        })
        .await
        .unwrap_err();
    for error in [archive_error, discard_error] {
        assert_eq!(error.code, crate::protocol::error_code::CONFLICT);
    }
    let stored = host
        .catalog_get(replacement.terminal_id)
        .await
        .unwrap()
        .unwrap();
    assert_manifest_snapshot(&stored, &manifest_snapshot(&replacement));
    let current_digest = crate::model::TerminalManifestDigest::capture(&replacement).unwrap();
    assert_ne!(current_digest, original_digest);
    let current = TerminalLifecycleSelection::unconditional(replacement.terminal_id)
        .with_expected_manifest_sha256(Some(current_digest));
    host.validate_product_selection(&current).await.unwrap();
}

#[tokio::test]
async fn combined_fence_rejects_either_changed_authority_without_mutation() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let original = fixture_manifest(directory.path());
    let runtime = host_runtime_record(
        &host,
        original.terminal_id,
        directory.path().join(COMBINED_FENCE_SOCKET_NAME),
    );
    persist_host_session(&host, &original, &runtime);
    let digest = crate::model::TerminalManifestDigest::capture(&original).unwrap();
    let catalog_only = TerminalLifecycleSelection::unconditional(original.terminal_id)
        .with_expected_manifest_sha256(Some(digest.clone()));
    assert!(lifecycle_discard_inactive_only(&catalog_only));
    let selected = TerminalLifecycleSelection::at_runtime_epoch(original.terminal_id, runtime.runtime_epoch)
        .with_expected_manifest_sha256(Some(digest));
    assert!(!lifecycle_discard_inactive_only(&selected));
    let exited = selected.clone();
    let exited = TerminalLifecycleSelection {
        expected_status: Some(TerminalRuntimeStatus::Exited),
        ..exited
    };
    assert!(lifecycle_discard_inactive_only(&exited));
    host.revalidate_lifecycle_fence(&selected).await.unwrap();

    let mut changed_manifest = original.clone();
    changed_manifest.name = COMBINED_FENCE_REPLACEMENT_NAME.into();
    host.catalog_upsert(&changed_manifest).await.unwrap();
    assert_eq!(
        host.revalidate_lifecycle_fence(&selected)
            .await
            .unwrap_err()
            .code,
        crate::protocol::error_code::CONFLICT
    );
    assert_eq!(
        read_json::<RuntimeRecord>(&host.paths.runtime_record(original.terminal_id)).unwrap(),
        runtime
    );

    host.catalog_upsert(&original).await.unwrap();
    let mut changed_runtime = runtime.clone();
    changed_runtime.runtime_epoch = Uuid::new_v4();
    atomic_write_json(
        &host.paths.runtime_record(original.terminal_id),
        &changed_runtime,
    )
    .unwrap();
    assert_eq!(
        host.revalidate_lifecycle_fence(&selected)
            .await
            .unwrap_err()
            .code,
        crate::protocol::error_code::CONFLICT
    );
    let stored = host
        .catalog_get(original.terminal_id)
        .await
        .unwrap()
        .unwrap();
    assert_manifest_snapshot(&stored, &manifest_snapshot(&original));
}

#[tokio::test]
async fn epoch_fence_rejects_replacement_after_validation_before_termination_rpc() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    let original = host_runtime_record(
        &host,
        manifest.terminal_id,
        directory.path().join(LIFECYCLE_ORIGINAL_SOCKET_NAME),
    );
    persist_host_session(&host, &manifest, &original);
    let runtime_path = host.paths.runtime_record(manifest.terminal_id);
    let replacement_socket = directory.path().join(LIFECYCLE_REPLACEMENT_SOCKET_NAME);
    let replacement_listener = UnixListener::bind(&replacement_socket).unwrap();
    let mut replacement = original.clone();
    replacement.runtime_epoch = Uuid::new_v4();
    replacement.worker_id = Uuid::new_v4();
    replacement.socket_path = replacement_socket;
    replacement.auth_token = LIFECYCLE_REPLACEMENT_AUTH_TOKEN.into();
    let replacement_for_hook = replacement.clone();
    let runtime_path_for_hook = runtime_path.clone();
    *lock_recover(&host.catalog_test_probes.lifecycle_revalidation) = Some(Box::new(move || {
        atomic_write_json(&runtime_path_for_hook, &replacement_for_hook).unwrap();
    }));
    let selection =
        TerminalLifecycleSelection::at_runtime_epoch(manifest.terminal_id, original.runtime_epoch);

    let error = tokio::time::timeout(
        HOST_BLOCKING_TEST_TIMEOUT,
        host.terminate_product_request(TerminalTerminateRequest {
            selection,
            force: true,
        }),
    )
    .await
    .unwrap()
    .unwrap_err();

    assert_eq!(error.code, crate::protocol::error_code::CONFLICT);
    assert_eq!(
        read_json::<RuntimeRecord>(&runtime_path).unwrap(),
        replacement
    );
    assert_listener_received_no_connection(replacement_listener);
    assert!(!host.paths.terminate_tombstone(manifest.terminal_id).exists());
}

fn assert_listener_received_no_connection(listener: UnixListener) {
    let listener = listener.into_std().unwrap();
    assert!(matches!(
        listener.accept(),
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock
    ));
}

const SAME_CLOCK_REPLACEMENT_NAME: &str = "same-clock replacement";
const COMBINED_FENCE_SOCKET_NAME: &str = "combined-fence.sock";
const COMBINED_FENCE_REPLACEMENT_NAME: &str = "combined fence replacement";
const LIFECYCLE_ORIGINAL_SOCKET_NAME: &str = "lifecycle-original.sock";
const LIFECYCLE_REPLACEMENT_SOCKET_NAME: &str = "lifecycle-replacement.sock";
const LIFECYCLE_REPLACEMENT_AUTH_TOKEN: &str = "lifecycle-replacement-token";

include!("lifecycle_runtime_absence_fence.rs");
include!("lifecycle_exact_termination_fence.rs");
