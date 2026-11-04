use crate::host::tests::*;
use super::*;

const ARCHIVE_SAFETY_SOCKET_NAME: &str = "archive-safety.sock";
const ARCHIVE_REPLACEMENT_SOCKET_NAME: &str = "archive-replacement.sock";
const ARCHIVE_REMOTE_SOCKET_NAME: &str = "archive-remote.sock";
const ARCHIVE_REMOTE_IDENTITY_DETAIL: &str = "archive remote identity rejection";
const ARCHIVE_FINALIZED_CHILD_REQUIRED: &str =
    "replacement-socket scenario requires a finalized child before archive";

#[test]
fn archive_treats_an_unverifiable_child_identity_as_potentially_live() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let terminal_id = Uuid::new_v4();
    let mut record = dead_archive_record(
        &host,
        terminal_id,
        directory.path().join(ARCHIVE_SAFETY_SOCKET_NAME),
    );
    record.child_start_time = u64::default();
    persist_archive_runtime(&host, &record);

    assert!(host.runtime_record_may_be_live(terminal_id));
}

#[tokio::test]
async fn archive_rejects_worker_exit_while_agent_child_remains_live() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let mut worker = ArchiveTestProcess::spawn();
    let child = ArchiveTestProcess::spawn();
    let mut record = host_runtime_record(
        &host,
        Uuid::new_v4(),
        directory.path().join(ARCHIVE_SAFETY_SOCKET_NAME),
    );
    bind_archive_processes(&mut record, &worker, &child);
    persist_archive_runtime(&host, &record);
    persist_archive_exit(&host, &record, record.runtime_epoch);
    worker.stop();

    let error = host
        .settle_archive_runtime_inner(&record, ARCHIVE_TEST_TIMEOUT)
        .await
        .unwrap_err();

    assert_eq!(error.code, crate::protocol::error_code::WORKER_UNREACHABLE);
    assert_eq!(error.message, ARCHIVE_RUNTIME_UNSETTLED);
    assert!(process::process_identity_matches(
        record.child_pid,
        record.child_start_time
    ));
}

#[tokio::test]
async fn archive_rejects_missing_or_mismatched_durable_exit() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let record = dead_archive_record(
        &host,
        Uuid::new_v4(),
        directory.path().join(ARCHIVE_SAFETY_SOCKET_NAME),
    );
    persist_archive_runtime(&host, &record);
    let missing = host
        .settle_archive_runtime_inner(&record, ARCHIVE_TEST_TIMEOUT)
        .await
        .unwrap_err();
    assert_eq!(missing.message, ARCHIVE_RUNTIME_UNSETTLED);

    persist_archive_exit(&host, &record, Uuid::new_v4());
    let mismatched = host
        .settle_archive_runtime_inner(&record, ARCHIVE_TEST_TIMEOUT)
        .await
        .unwrap_err();
    assert_eq!(mismatched.message, ARCHIVE_RUNTIME_UNSETTLED);
}

#[tokio::test]
async fn archive_rejects_replaced_runtime_identity_after_status_loss() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let record = dead_archive_record(
        &host,
        Uuid::new_v4(),
        directory.path().join(ARCHIVE_SAFETY_SOCKET_NAME),
    );
    persist_archive_runtime(&host, &record);
    persist_archive_exit(&host, &record, record.runtime_epoch);
    let mut replacement = record.clone();
    replacement.worker_id = Uuid::new_v4();
    persist_archive_runtime(&host, &replacement);

    let error = host
        .settle_archive_runtime_inner(&record, ARCHIVE_TEST_TIMEOUT)
        .await
        .unwrap_err();

    assert_eq!(error.message, ARCHIVE_RUNTIME_UNSETTLED);
}

#[tokio::test]
async fn archive_preserves_nonretryable_remote_status_error() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let socket = directory.path().join(ARCHIVE_REMOTE_SOCKET_NAME);
    let listener = UnixListener::bind(&socket).unwrap();
    let record = host_runtime_record(&host, Uuid::new_v4(), socket);
    let server_record = record.clone();
    let server = tokio::spawn(async move {
        let (mut stream, request) = accept_archive_request(&listener, &server_record).await;
        let error = ErrorBody::new(
            crate::protocol::error_code::IDENTITY_MISMATCH,
            ARCHIVE_REMOTE_IDENTITY_DETAIL,
            false,
        );
        stream
            .respond(&ResponseEnvelope::failure(&request, error))
            .await
            .unwrap();
    });

    let error = host
        .settle_archive_runtime(&record, PROCESS_EXIT_TIMEOUT)
        .await
        .unwrap_err();

    assert_eq!(error.code, crate::protocol::error_code::IDENTITY_MISMATCH);
    assert_eq!(error.message, ARCHIVE_REMOTE_IDENTITY_DETAIL);
    assert!(!error.retryable);
    server.await.unwrap();
}

#[tokio::test]
async fn archive_does_not_swallow_replacement_socket_retirement_error() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let socket = directory.path().join(ARCHIVE_REPLACEMENT_SOCKET_NAME);
    let mut worker = ArchiveWorkerProcessFixture::spawn(
        socket.clone(),
        ArchiveWorkerFixtureMode::ExitedThenShutdownReplacement,
    );
    let mut record = host_runtime_record(&host, Uuid::new_v4(), socket);
    worker.configure(&mut record);
    persist_archive_runtime(&host, &record);
    persist_archive_exit(&host, &record, record.runtime_epoch);
    assert_eq!(
        worker.child_identity().observe(),
        process::ProcessIdentityObservation::Gone,
        "{ARCHIVE_FINALIZED_CHILD_REQUIRED}"
    );

    let error = host
        .settle_archive_runtime(&record, PROCESS_EXIT_TIMEOUT)
        .await
        .unwrap_err();

    assert_eq!(error.code, crate::protocol::error_code::WORKER_UNREACHABLE);
    assert_eq!(error.message, WORKER_RETIREMENT_SOCKET_ACTIVE);
    worker.reap_completed_worker();
    worker.release_replacement();
}

include!("archive_retired_custody.rs");
