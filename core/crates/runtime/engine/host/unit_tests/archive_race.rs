use crate::host::tests::*;
use super::*;

const ARCHIVE_MISSING_SOCKET_NAME: &str = "archive-missing.sock";

#[tokio::test]
async fn archive_accepts_selected_worker_exit_after_status_socket_loss() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let mut worker = ArchiveTestProcess::spawn();
    let mut child = ArchiveTestProcess::spawn();
    let mut record = host_runtime_record(
        &host,
        Uuid::new_v4(),
        directory.path().join(ARCHIVE_MISSING_SOCKET_NAME),
    );
    bind_archive_processes(&mut record, &worker, &child);
    persist_archive_runtime(&host, &record);
    persist_archive_exit(&host, &record, record.runtime_epoch);
    assert!(process::process_identity_matches(
        record.worker_pid,
        record.worker_start_time
    ));

    let mut settlement = Box::pin(host.settle_archive_runtime(&record, PROCESS_EXIT_TIMEOUT));
    let pending = matches!(futures::poll!(&mut settlement), std::task::Poll::Pending);
    worker.stop();
    child.stop();

    assert!(pending);
    assert!(settlement.await.is_ok());
    assert!(!process::process_identity_matches(
        record.worker_pid,
        record.worker_start_time
    ));
}

#[tokio::test]
async fn archive_status_loss_for_live_worker_is_retryable_not_internal() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let record = host_runtime_record(
        &host,
        Uuid::new_v4(),
        directory.path().join(ARCHIVE_MISSING_SOCKET_NAME),
    );

    let error = host
        .settle_archive_runtime(&record, ARCHIVE_TEST_TIMEOUT)
        .await
        .unwrap_err();

    assert_eq!(error.code, crate::protocol::error_code::WORKER_UNREACHABLE);
    assert!(error.retryable);
    assert_ne!(error.code, crate::protocol::error_code::INTERNAL);
}
