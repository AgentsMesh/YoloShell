use crate::host::tests::*;
use super::*;

const RELEASED_PROXY_REPLACEMENT_SOCKET_NAME: &str = "released-proxy-replacement.sock";
const RELEASED_PROXY_REPLACEMENT_AUTH_TOKEN: &str = "released-proxy-replacement-token";
const RELEASED_PROXY_CONTROLLER_GENERATION_INCREMENT: u64 = 1;

#[derive(Clone, Copy)]
enum ReleasedProxyMutation {
    AuthToken,
    SocketPath,
    ControllerGeneration,
    ExitEpoch,
}

#[tokio::test]
async fn released_attachment_revalidates_durable_authority_after_worker_response() {
    for mutation in [
        ReleasedProxyMutation::AuthToken,
        ReleasedProxyMutation::SocketPath,
        ReleasedProxyMutation::ControllerGeneration,
        ReleasedProxyMutation::ExitEpoch,
    ] {
        assert_released_proxy_rejects_replacement(mutation).await;
    }
}

async fn assert_released_proxy_rejects_replacement(mutation: ReleasedProxyMutation) {
    let (directory, host, record) = released_exit_fixture(ExitReason::Natural);
    let listener = UnixListener::bind(&record.socket_path).unwrap();
    let runtime_path = host.paths.runtime_record(record.terminal_id);
    let exit_path = host.paths.exit_record(record.terminal_id);
    let replacement_socket = directory
        .path()
        .join(RELEASED_PROXY_REPLACEMENT_SOCKET_NAME);
    let server_record = record.clone();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream = UnixStream::from(stream);
        let mut stream = accept_worker_auth(stream, &server_record).await;
        let request: RequestEnvelope = stream.request().await.unwrap();
        replace_released_proxy_evidence(
            mutation,
            &runtime_path,
            &exit_path,
            &server_record,
            replacement_socket,
        );
        let initial = empty_attachment(server_record.runtime_epoch);
        let response = ResponseEnvelope::success(&request, &initial).unwrap();
        stream.respond(&response).await.unwrap();
    });
    let error = host
        .rpc_attachment(&request(
            method::ATTACH_SESSION,
            &AttachRequest {
                terminal_id: record.terminal_id,
                after_seq: RACE_ATTACH_AFTER_SEQUENCE,
            },
        ))
        .await
        .err()
        .unwrap();
    assert_eq!(error.code, crate::protocol::error_code::IDENTITY_MISMATCH);
    assert!(!error.retryable);
    server.await.unwrap();
}

fn replace_released_proxy_evidence(
    mutation: ReleasedProxyMutation,
    runtime_path: &Path,
    exit_path: &Path,
    authenticated: &RuntimeRecord,
    replacement_socket: PathBuf,
) {
    if matches!(mutation, ReleasedProxyMutation::ExitEpoch) {
        let mut exit = read_json::<ExitRecord>(exit_path).unwrap();
        exit.runtime_epoch = Uuid::new_v4();
        atomic_write_json(exit_path, &exit).unwrap();
        return;
    }
    let mut replacement = authenticated.clone();
    match mutation {
        ReleasedProxyMutation::AuthToken => {
            replacement.auth_token = RELEASED_PROXY_REPLACEMENT_AUTH_TOKEN.into();
        }
        ReleasedProxyMutation::SocketPath => replacement.socket_path = replacement_socket,
        ReleasedProxyMutation::ControllerGeneration => {
            replacement.controller_generation = replacement
                .controller_generation
                .saturating_add(RELEASED_PROXY_CONTROLLER_GENERATION_INCREMENT);
        }
        ReleasedProxyMutation::ExitEpoch => unreachable!(),
    }
    atomic_write_json(runtime_path, &replacement).unwrap();
}
