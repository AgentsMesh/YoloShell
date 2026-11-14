use crate::host::tests::*;
use super::*;

const COMPLETED_REPLACEMENT_AUTH_TOKEN: &str = "completed-replacement-token";
const COMPLETED_REPLACEMENT_SOCKET_NAME: &str = "completed-replacement.sock";
const COMPLETED_REPLACEMENT_GENERATION_INCREMENT: u64 = 1;

#[derive(Clone, Copy, Debug)]
enum CompletedActiveAuthorityMutation {
    AuthToken,
    ControllerGeneration,
    SocketPath,
}

#[tokio::test]
async fn completed_attachment_active_authority_rechecks_durable_runtime() {
    for mutation in [
        CompletedActiveAuthorityMutation::AuthToken,
        CompletedActiveAuthorityMutation::ControllerGeneration,
        CompletedActiveAuthorityMutation::SocketPath,
    ] {
        assert_completed_active_authority_replacement_rejected(mutation).await;
    }
}

async fn assert_completed_active_authority_replacement_rejected(
    mutation: CompletedActiveAuthorityMutation,
) {
    let ActiveCompletedFixture {
        _directory: directory,
        host,
        record,
        manifest,
        runtime,
        listener,
        ..
    } = active_completed_fixture();
    lock_recover(&host.catalog).insert(&manifest).unwrap();
    let runtime_path = host.paths.runtime_record(record.terminal_id);
    let replacement = completed_active_authority_replacement(
        mutation,
        &record,
        directory.path().join(COMPLETED_REPLACEMENT_SOCKET_NAME),
    );
    let server_record = record.clone();
    let server_runtime = runtime.clone();
    let (dispatch_finished, await_dispatch_finished) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream =
            accept_completed_worker_auth(UnixStream::from(stream), &server_record).await;
        let status: RequestEnvelope = stream.request().await.unwrap();
        assert_eq!(status.method, WORKER_STATUS_METHOD);
        atomic_write_json(&runtime_path, &replacement).unwrap();
        stream
            .respond(&ResponseEnvelope::success(&status, &server_runtime).unwrap())
            .await
            .unwrap();
        tokio::select! {
            biased;
            additional = listener.accept() => panic!(
                "completed authority mutation {mutation:?} caused an additional Worker RPC: {additional:?}"
            ),
            finished = await_dispatch_finished => finished.unwrap(),
        }
    });

    let response = host
        .dispatch(&request(
            method::GET_SESSION,
            &TerminalRef {
                terminal_id: record.terminal_id,
            },
        ))
        .await;
    let _ = dispatch_finished.send(());

    assert!(response.ok, "{:?}", response.error);
    let view: TerminalView = response.decode_payload().unwrap();
    assert!(view.completed_attachment.is_none());
    await_completed_worker(server).await;
}

fn completed_active_authority_replacement(
    mutation: CompletedActiveAuthorityMutation,
    record: &RuntimeRecord,
    replacement_socket: PathBuf,
) -> RuntimeRecord {
    let mut replacement = record.clone();
    match mutation {
        CompletedActiveAuthorityMutation::AuthToken => {
            replacement.auth_token = COMPLETED_REPLACEMENT_AUTH_TOKEN.into();
        }
        CompletedActiveAuthorityMutation::ControllerGeneration => {
            replacement.controller_generation = replacement
                .controller_generation
                .checked_add(COMPLETED_REPLACEMENT_GENERATION_INCREMENT)
                .unwrap();
        }
        CompletedActiveAuthorityMutation::SocketPath => {
            replacement.socket_path = replacement_socket;
        }
    }
    replacement
}

#[tokio::test]
async fn completed_attachment_snapshot_rechecks_released_exit() {
    let (_directory, host, record) = released_exit_fixture(ExitReason::Natural);
    let listener = UnixListener::bind(&record.socket_path).unwrap();
    let exit_path = host.paths.exit_record(record.terminal_id);
    let exit = read_json(&exit_path).unwrap();
    let expected = completed_test_snapshot(&exit);
    let server_record = record.clone();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream =
            accept_completed_worker_auth(UnixStream::from(stream), &server_record).await;
        let snapshot: RequestEnvelope = stream.request().await.unwrap();
        let mut replaced_exit: ExitRecord = read_json(&exit_path).unwrap();
        replaced_exit.runtime_epoch = Uuid::new_v4();
        atomic_write_json(&exit_path, &replaced_exit).unwrap();
        stream
            .respond(&ResponseEnvelope::success(&snapshot, &expected).unwrap())
            .await
            .unwrap();
    });
    let snapshot_request = crate::worker::SnapshotRequest {
        terminal_id: record.terminal_id,
        runtime_epoch: record.runtime_epoch,
    };

    let error = host
        .proxy_runtime_bound_worker_request(&request(method::GET_SNAPSHOT, &snapshot_request))
        .await
        .unwrap_err();

    assert_eq!(error.code, crate::protocol::error_code::IDENTITY_MISMATCH);
    assert!(!error.retryable);
    await_completed_worker(server).await;
}
