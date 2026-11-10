use crate::host::tests::*;
fn absent_runtime_selection(manifest: &TerminalManifest) -> TerminalLifecycleSelection {
    TerminalLifecycleSelection::selected(
        manifest.terminal_id,
        manifest.updated_at_ms,
        TerminalRuntimeStatus::Unreachable,
    )
    .with_expected_manifest_sha256(Some(
        crate::model::TerminalManifestDigest::capture(manifest).unwrap(),
    ))
}

fn arm_absent_runtime_replacement(
    host: &Host,
    manifest: &TerminalManifest,
    socket_name: &str,
) -> (RuntimeRecord, UnixListener) {
    let socket = host.paths.home().join(socket_name);
    let listener = UnixListener::bind(&socket).unwrap();
    let replacement = host_runtime_record(host, manifest.terminal_id, socket);
    let replacement_for_hook = replacement.clone();
    let runtime_path = host.paths.runtime_record(manifest.terminal_id);
    *lock_recover(&host.catalog_test_probes.lifecycle_revalidation) = Some(Box::new(move || {
        atomic_write_json(&runtime_path, &replacement_for_hook).unwrap();
    }));
    (replacement, listener)
}

fn assert_absent_runtime_replacement_survived(
    host: &Host,
    replacement: &RuntimeRecord,
    listener: UnixListener,
) {
    assert_eq!(
        read_json::<RuntimeRecord>(&host.paths.runtime_record(replacement.terminal_id)).unwrap(),
        *replacement
    );
    assert_listener_received_no_connection(listener);
    assert!(
        !host
            .paths
            .terminate_tombstone(replacement.terminal_id)
            .exists()
    );
}

#[tokio::test]
async fn absent_runtime_fence_rejects_replacement_before_stop() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    seed_recovery_manifest(&host, &manifest).await;
    let selection = absent_runtime_selection(&manifest);
    let (replacement, listener) =
        arm_absent_runtime_replacement(&host, &manifest, ABSENT_STOP_SOCKET_NAME);

    let error = host
        .terminate_product_request(TerminalTerminateRequest {
            selection,
            force: true,
        })
        .await
        .unwrap_err();

    assert_eq!(error.code, crate::protocol::error_code::CONFLICT);
    assert_absent_runtime_replacement_survived(&host, &replacement, listener);
}

#[tokio::test]
async fn absent_runtime_fence_rejects_replacement_before_archive() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    seed_recovery_manifest(&host, &manifest).await;
    let selection = absent_runtime_selection(&manifest);
    let (replacement, listener) =
        arm_absent_runtime_replacement(&host, &manifest, ABSENT_ARCHIVE_SOCKET_NAME);

    let error = host.archive_product_session(selection).await.unwrap_err();

    assert_eq!(error.code, crate::protocol::error_code::CONFLICT);
    assert_absent_runtime_replacement_survived(&host, &replacement, listener);
    assert!(
        !host
            .catalog_get(manifest.terminal_id)
            .await
            .unwrap()
            .unwrap()
            .archived
    );
}

#[tokio::test]
async fn absent_runtime_fence_rejects_replacement_before_discard() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    seed_recovery_manifest(&host, &manifest).await;
    let selection = absent_runtime_selection(&manifest);
    let (replacement, listener) =
        arm_absent_runtime_replacement(&host, &manifest, ABSENT_DISCARD_SOCKET_NAME);

    let error = host
        .discard_product_session(TerminalDiscardRequest {
            selection,
            force: true,
        })
        .await
        .unwrap_err();

    assert_eq!(error.code, crate::protocol::error_code::CONFLICT);
    assert_absent_runtime_replacement_survived(&host, &replacement, listener);
    assert!(
        !host
            .catalog_get(manifest.terminal_id)
            .await
            .unwrap()
            .unwrap()
            .archived
    );
}

#[test]
fn absent_runtime_fence_rejects_dangling_runtime_aliases() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let terminal_id = Uuid::new_v4();
    host.paths.ensure_session(terminal_id).unwrap();
    std::os::unix::fs::symlink(
        directory.path().join(ABSENT_RUNTIME_ALIAS_TARGET),
        host.paths.runtime_record(terminal_id),
    )
    .unwrap();

    let error = host
        .require_current_lifecycle_runtime(terminal_id, &LifecycleRuntimeFence::Absent)
        .unwrap_err();

    assert_eq!(error.code, crate::protocol::error_code::CONFLICT);
}

#[test]
fn archive_fence_rejects_pending_runtime_publication_artifacts() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let terminal_id = Uuid::new_v4();
    host.paths.ensure_session(terminal_id).unwrap();
    write_private_bytes(
        &host.paths.worker_startup_checkpoint(terminal_id),
        ABSENT_PENDING_CHECKPOINT_BYTES,
    );

    let error = host
        .require_no_pending_lifecycle_runtime(terminal_id, &LifecycleRuntimeFence::Absent)
        .unwrap_err();

    assert_eq!(error.code, crate::protocol::error_code::CONFLICT);
}

const ABSENT_STOP_SOCKET_NAME: &str = "absent-stop-replacement.sock";
const ABSENT_ARCHIVE_SOCKET_NAME: &str = "absent-archive-replacement.sock";
const ABSENT_DISCARD_SOCKET_NAME: &str = "absent-discard-replacement.sock";
const ABSENT_RUNTIME_ALIAS_TARGET: &str = "missing-runtime-target";
const ABSENT_PENDING_CHECKPOINT_BYTES: &[u8] = b"pending-runtime-publication";

include!("lifecycle_archive_publication_fence.rs");
include!("lifecycle_runtime_absence_strict.rs");
