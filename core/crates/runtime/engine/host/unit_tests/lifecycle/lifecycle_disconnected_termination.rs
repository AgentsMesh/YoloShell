use crate::host::tests::*;
use super::*;

const DISCONNECTED_TERMINATION_SOCKET_NAME: &str = "disconnected-termination.sock";

#[tokio::test]
async fn disconnected_termination_waits_for_exact_runtime_settlement() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let terminal_id = Uuid::new_v4();
    let mut worker = ArchiveTestProcess::spawn();
    let mut child = ArchiveTestProcess::spawn();
    let mut record = host_runtime_record(
        &host,
        terminal_id,
        directory.path().join(DISCONNECTED_TERMINATION_SOCKET_NAME),
    );
    bind_archive_processes(&mut record, &worker, &child);
    assert!(process::process_identity_matches(
        record.worker_pid,
        record.worker_start_time
    ));
    assert!(process::process_identity_matches(
        record.child_pid,
        record.child_start_time
    ));
    persist_archive_runtime(&host, &record);
    persist_archive_exit(&host, &record, record.runtime_epoch);

    let mut settlement =
        Box::pin(host.settle_unreachable_runtime_termination(terminal_id, false, None));
    #[cfg(not(target_os = "linux"))]
    assert!(matches!(
        futures::poll!(&mut settlement),
        std::task::Poll::Pending
    ));
    #[cfg(target_os = "linux")]
    assert!(matches!(
        futures::poll!(&mut settlement),
        std::task::Poll::Ready(Ok(false))
    ));
    worker.stop();
    child.stop();
    #[cfg(not(target_os = "linux"))]
    assert!(settlement.await.unwrap());
    #[cfg(target_os = "linux")]
    {
        drop(settlement);
        crate::host::linux_retirement_evidence::publish(&host, &record);
        assert!(
            host.settle_unreachable_runtime_termination(terminal_id, false, None)
                .await
                .unwrap()
        );
    }
    let tombstone: crate::model::TerminationTombstone =
        read_json(&host.paths.terminate_tombstone(terminal_id)).unwrap();
    assert_eq!(tombstone.runtime_epoch, record.runtime_epoch);
    assert!(!tombstone.force);
}

#[tokio::test]
async fn disconnected_termination_requires_durable_exit_proof() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let terminal_id = Uuid::new_v4();
    let mut worker = ArchiveTestProcess::spawn();
    let mut child = ArchiveTestProcess::spawn();
    let mut record = host_runtime_record(
        &host,
        terminal_id,
        directory.path().join(DISCONNECTED_TERMINATION_SOCKET_NAME),
    );
    bind_archive_processes(&mut record, &worker, &child);
    persist_archive_runtime(&host, &record);
    worker.stop();
    child.stop();

    let result = host
        .settle_unreachable_runtime_termination(terminal_id, false, None)
        .await;
    #[cfg(not(target_os = "linux"))]
    assert!(!result.unwrap());
    #[cfg(target_os = "linux")]
    {
        let error = result.unwrap_err();
        assert_eq!(
            error.code,
            crate::protocol::error_code::DESTRUCTIVE_SESSION_UNAVAILABLE
        );
        assert!(error.retryable);
    }
    assert!(!host.paths.terminate_tombstone(terminal_id).exists());
}

#[tokio::test]
async fn disconnected_runtime_accepts_preexisting_durable_termination_proof() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let terminal_id = Uuid::new_v4();
    let mut worker = ArchiveTestProcess::spawn();
    let mut child = ArchiveTestProcess::spawn();
    let mut record = host_runtime_record(
        &host,
        terminal_id,
        directory.path().join(DISCONNECTED_TERMINATION_SOCKET_NAME),
    );
    bind_archive_processes(&mut record, &worker, &child);
    persist_archive_runtime(&host, &record);
    let tombstone =
        crate::model::TerminationTombstone::user_request(terminal_id, record.runtime_epoch, true);
    atomic_write_json(&host.paths.terminate_tombstone(terminal_id), &tombstone).unwrap();
    worker.stop();
    child.stop();

    assert!(host.runtime_is_settled(&record, PROCESS_EXIT_TIMEOUT).await);
    assert!(!host.paths.exit_record(terminal_id).exists());
}
