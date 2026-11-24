use crate::host::tests::*;
use super::*;

#[tokio::test]
async fn idle_shutdown_rejects_attachments_and_reopens_the_drain() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let attachment = crate::ingress::try_admit(&host.attachment_clients).unwrap();

    let error = host.prepare_idle_shutdown().await.err().unwrap();
    assert_eq!(error.code, crate::protocol::error_code::CONFLICT);
    assert!(!host.request_drain.is_draining());

    drop(attachment);
    let preparation = host.prepare_idle_shutdown().await.unwrap();
    drop(preparation);
    assert!(!host.request_drain.is_draining());
}

#[tokio::test]
async fn idle_shutdown_rejects_creation_recovery_and_indeterminate_sessions() {
    let creation_directory = tempdir().unwrap();
    let creation_host = Host::open(HostConfig::new(creation_directory.path())).unwrap();
    let request = interleaving_request(creation_directory.path(), Uuid::new_v4());
    reserve_interleaving(&creation_host, &request);
    assert!(creation_host.prepare_idle_shutdown().await.is_err());
    assert!(!creation_host.request_drain.is_draining());

    let session_directory = tempdir().unwrap();
    let session_host = Host::open(HostConfig::new(session_directory.path())).unwrap();
    let mut manifest = fixture_manifest(session_directory.path());
    manifest.recovery_state = RecoveryState::Uncertain;
    seed_recovery_manifest(&session_host, &manifest).await;
    assert!(session_host.prepare_idle_shutdown().await.is_err());
    assert!(!session_host.request_drain.is_draining());
}

#[tokio::test]
async fn archived_rows_without_a_runtime_do_not_block_idle_shutdown() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let mut manifest = fixture_manifest(directory.path());
    manifest.archived = true;
    manifest.recovery_state = RecoveryState::Uncertain;
    seed_recovery_manifest(&host, &manifest).await;

    let preparation = host.prepare_idle_shutdown().await.unwrap();
    drop(preparation);
}

#[tokio::test]
async fn archived_settled_runtime_does_not_block_idle_shutdown() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let mut manifest = fixture_manifest(directory.path());
    manifest.archived = true;
    manifest.recovery_state = RecoveryState::Uncertain;
    seed_recovery_manifest(&host, &manifest).await;
    let record = dead_archive_record(
        &host,
        manifest.terminal_id,
        directory.path().join(ARCHIVED_SETTLED_SOCKET),
    );
    persist_archive_runtime(&host, &record);

    let preparation = host.prepare_idle_shutdown().await.unwrap();
    drop(preparation);
}

#[tokio::test]
async fn archived_live_runtime_blocks_idle_shutdown() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let mut manifest = fixture_manifest(directory.path());
    manifest.archived = true;
    manifest.recovery_state = RecoveryState::Uncertain;
    seed_recovery_manifest(&host, &manifest).await;
    let record = host_runtime_record(
        &host,
        manifest.terminal_id,
        directory.path().join(ARCHIVED_LIVE_SOCKET),
    );
    persist_archive_runtime(&host, &record);

    let error = host.prepare_idle_shutdown().await.err().unwrap();
    assert_eq!(error.code, crate::protocol::error_code::CONFLICT);
    assert!(!host.request_drain.is_draining());
}

#[tokio::test]
async fn pending_recovery_write_blocks_idle_shutdown_until_it_is_retried() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let mut manifest = fixture_manifest(directory.path());
    let terminal_id = manifest.terminal_id;
    manifest.archived = true;
    seed_recovery_manifest(&host, &manifest).await;
    block_recovery_manifest_write(&host, terminal_id);
    assert!(!host.persist_recovery_state_durable(&mut manifest, RecoveryState::Created));
    assert_eq!(lock_recover(&host.pending_recovery_writes).len(), 1);
    assert!(host.prepare_idle_shutdown().await.is_err());

    std::fs::remove_dir(host.paths.manifest(terminal_id)).unwrap();
    host.write_namespace_json(&host.paths.manifest(terminal_id), &manifest)
        .unwrap();
    assert_eq!(host.retry_pending_recovery_writes().await, 0);
    assert!(lock_recover(&host.pending_recovery_writes).is_empty());
    let preparation = host.prepare_idle_shutdown().await.unwrap();
    drop(preparation);
}

#[tokio::test]
async fn settled_archived_row_without_runtime_permits_idle_shutdown() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let mut manifest = fixture_manifest(directory.path());
    manifest.archived = true;
    seed_recovery_manifest(&host, &manifest).await;

    let preparation = host.prepare_idle_shutdown().await.unwrap();
    drop(preparation);
}

#[tokio::test]
async fn request_drain_waits_for_mutation_and_rejects_new_admission() {
    let drain = request_drain::HostRequestDrain::default();
    let mutation = drain.admit_mutation().await.unwrap();
    let contender = drain.clone();
    let preparing = tokio::spawn(async move { contender.prepare().await.unwrap() });
    tokio::task::yield_now().await;

    assert!(!drain.is_draining());
    drop(mutation);
    let preparation = preparing.await.unwrap();
    assert!(drain.is_draining());
    assert!(drain.admit_mutation().await.is_err());
    drop(preparation);
    assert!(drain.admit_mutation().await.is_ok());
}

#[tokio::test]
async fn cancelling_a_queued_drain_cannot_leave_admission_closed() {
    let drain = request_drain::HostRequestDrain::default();
    let mutation = drain.admit_mutation().await.unwrap();
    let contender = drain.clone();
    let preparing = tokio::spawn(async move { contender.prepare().await });
    tokio::task::yield_now().await;

    preparing.abort();
    let joined = preparing.await;
    assert!(matches!(joined, Err(error) if error.is_cancelled()));
    assert!(!drain.is_draining());
    drop(mutation);
    assert!(drain.admit_mutation().await.is_ok());
}

#[tokio::test]
async fn exited_catalog_rows_permit_idle_shutdown() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    let record = dead_archive_record(
        &host,
        manifest.terminal_id,
        directory.path().join(IDLE_SHUTDOWN_EXITED_SOCKET),
    );
    persist_host_session(&host, &manifest, &record);
    persist_archive_exit(&host, &record, record.runtime_epoch);

    let preparation = host.prepare_idle_shutdown().await.unwrap();
    drop(preparation);
}

#[tokio::test]
async fn quiescent_shutdown_fences_new_clients_before_commit() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let clients = std::sync::Arc::new(tokio::sync::Semaphore::new(MAX_HOST_CLIENTS));

    let preparation = host
        .prepare_quiescent_shutdown(std::sync::Arc::clone(&clients))
        .await
        .unwrap();

    assert!(clients.clone().try_acquire_owned().is_err());
    preparation.commit();
    tokio::time::timeout(HOST_REQUEST_TIMEOUT, host.request_drain.wait_for_shutdown())
        .await
        .unwrap()
        .unwrap();
}


include!("idle_shutdown_admission.rs");
