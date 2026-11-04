use crate::host::tests::*;
use super::*;

const RELEASED_RETIREMENT_SOCKET: &str = "released-retirement.sock";
const RELEASED_RETIREMENT_FAILED: &str = "released worker retirement did not converge";

#[tokio::test]
async fn released_retirement_closes_acknowledged_control_before_waiting_for_worker_exit() {
    let (_directory, host, record, mut worker) =
        released_retirement_fixture(ArchiveWorkerFixtureMode::ReleasedShutdownThenEof);
    let before = std::fs::read(host.paths.exit_record(record.terminal_id)).unwrap();

    if let Err(error) = host.retire_released_worker_classified(&record).await {
        panic!(
            "{RELEASED_RETIREMENT_FAILED}: {}",
            error.into_error_body().message
        );
    }

    worker.reap_completed_worker();
    assert!(host.runtime_is_settled(&record, Duration::ZERO).await);
    assert_eq!(
        std::fs::read(host.paths.exit_record(record.terminal_id)).unwrap(),
        before
    );
}

#[tokio::test]
async fn released_retirement_preserves_rejection_even_when_peer_exits_after_control_eof() {
    let (_directory, host, record, mut worker) =
        released_retirement_fixture(ArchiveWorkerFixtureMode::RejectedShutdownThenEof);

    let error = host
        .retire_released_worker_classified(&record)
        .await
        .unwrap_err()
        .into_error_body();

    worker.reap_completed_worker();
    assert_eq!(error.code, crate::protocol::error_code::IDENTITY_MISMATCH);
    assert_eq!(error.message, ARCHIVE_WORKER_RELEASED_SHUTDOWN_REJECTED);
    assert!(!error.retryable);
}

fn released_retirement_fixture(
    mode: ArchiveWorkerFixtureMode,
) -> (
    tempfile::TempDir,
    Host,
    RuntimeRecord,
    ArchiveWorkerProcessFixture,
) {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let socket = directory.path().join(RELEASED_RETIREMENT_SOCKET);
    let worker = ArchiveWorkerProcessFixture::spawn(socket.clone(), mode);
    let mut record = host_runtime_record(&host, Uuid::new_v4(), socket);
    worker.configure(&mut record);
    persist_archive_runtime(&host, &record);
    persist_archive_exit(&host, &record, record.runtime_epoch);
    host.secondary_custody.revoke_rpc_test(&record);
    host.secondary_custody.install_test_retired_lease(
        &host,
        &record,
        crate::host::secondary_custody::TestRetiredCustodyState::Released,
    );
    assert_eq!(
        worker.child_identity().observe(),
        process::ProcessIdentityObservation::Gone
    );
    host.validate_released_terminal_worker_shutdown(&record)
        .unwrap();
    (directory, host, record, worker)
}
