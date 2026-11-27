use crate::host::tests::*;
use super::*;

const COMPLETED_FORBIDDEN_METHODS: &[&str] = &[
    method::ACQUIRE_WRITER,
    method::GET_WRITER_LEASE,
    method::TAKEOVER_WRITER,
    method::RENEW_WRITER,
    method::RELEASE_WRITER,
    method::TERMINAL_INPUT,
    method::RESIZE_TERMINAL,
    method::SIGNAL_SESSION,
];

#[tokio::test]
async fn completed_attachment_rejects_lease_and_mutation_rpc_canonically() {
    let (_directory, host, record) = released_exit_fixture(ExitReason::Natural);
    let target = crate::worker::RuntimeRef::new(record.terminal_id, record.runtime_epoch);
    assert_released_completed_rpc_rejection(&host, &target).await;
}

#[tokio::test]
async fn active_completed_attachment_rejects_every_control_rpc() {
    let ActiveCompletedFixture {
        _directory,
        host,
        record,
        listener,
        ..
    } = active_completed_fixture();
    let server = tokio::spawn(serve_completed_worker_rejections(
        listener,
        record.clone(),
        COMPLETED_FORBIDDEN_METHODS,
    ));
    let target = crate::worker::RuntimeRef::new(record.terminal_id, record.runtime_epoch);

    assert_active_completed_rpc_rejection(&host, &target).await;
    await_completed_worker(server).await;
}

#[tokio::test]
async fn completed_attachment_keeps_the_exact_snapshot_readable() {
    let (_directory, host, record) = released_exit_fixture(ExitReason::Natural);
    let listener = UnixListener::bind(&record.socket_path).unwrap();
    let exit = read_json(&host.paths.exit_record(record.terminal_id)).unwrap();
    let expected = completed_test_snapshot(&exit);
    let server_record = record.clone();
    let server_snapshot = expected.clone();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream =
            accept_completed_worker_auth(UnixStream::from(stream), &server_record).await;
        let snapshot: RequestEnvelope = stream.request().await.unwrap();
        assert_eq!(snapshot.method, method::GET_SNAPSHOT);
        stream
            .respond(&ResponseEnvelope::success(&snapshot, &server_snapshot).unwrap())
            .await
            .unwrap();
    });
    let snapshot_request = crate::worker::SnapshotRequest {
        terminal_id: record.terminal_id,
        runtime_epoch: record.runtime_epoch,
    };

    let response = host
        .proxy_runtime_bound_worker_request(&request(method::GET_SNAPSHOT, &snapshot_request))
        .await
        .unwrap();

    assert_eq!(response.request_id, TEST_REQUEST_ID);
    assert_eq!(response.method, method::GET_SNAPSHOT);
    let actual: crate::model::TerminalSnapshot = response.decode_payload().unwrap();
    assert_eq!(actual.runtime_epoch, expected.runtime_epoch);
    assert_eq!(actual.barrier_sequence, expected.barrier_sequence);
    assert_eq!(actual.cols, expected.cols);
    assert_eq!(actual.rows, expected.rows);
    assert_eq!(actual.data, expected.data);
    await_completed_worker(server).await;
}

async fn assert_released_completed_rpc_rejection(host: &Host, target: &crate::worker::RuntimeRef) {
    for method_name in COMPLETED_FORBIDDEN_METHODS {
        let error = host
            .proxy_runtime_bound_worker_request(&request(method_name, target))
            .await
            .unwrap_err();
        assert_completed_rpc_error(error);
    }
}

async fn assert_active_completed_rpc_rejection(host: &Host, target: &crate::worker::RuntimeRef) {
    for method_name in COMPLETED_FORBIDDEN_METHODS {
        let response = host.dispatch(&request(method_name, target)).await;
        assert!(!response.ok);
        assert_active_completed_rpc_error(response.error.unwrap());
    }
}

fn assert_active_completed_rpc_error(error: ErrorBody) {
    assert_eq!(
        error.code,
        crate::lease::constants::code::SESSION_NOT_RUNNING
    );
    assert!(!error.retryable);
    assert_eq!(error.details, None);
}

fn assert_completed_rpc_error(error: ErrorBody) {
    assert_eq!(
        error.code,
        crate::lease::constants::code::SESSION_NOT_RUNNING
    );
    assert_eq!(error.message, COMPLETED_RUNTIME_NOT_RUNNING);
    assert!(!error.retryable);
    assert_eq!(error.details, None);
}
