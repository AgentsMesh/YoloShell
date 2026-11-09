use crate::host::tests::*;
use super::*;

const REPLACED_SHUTDOWN_SOCKET_NAME: &str = "replaced-released-shutdown.sock";
const REPLACED_SHUTDOWN_AUTH_TOKEN: &str = "replaced-released-shutdown-token";
const SHUTDOWN_CONTROLLER_GENERATION_INCREMENT: u64 = 1;
const UNEXPECTED_SHUTDOWN_DETAIL: &str = "released shutdown crossed replaced durable evidence";

#[derive(Clone, Copy)]
enum ReleasedShutdownMutation {
    AuthToken,
    SocketPath,
    ControllerGeneration,
    ExitEpoch,
}

#[derive(Clone, Copy)]
enum ReleasedShutdownPath {
    Direct,
    Resume,
}

#[tokio::test]
async fn released_shutdown_revalidates_durable_authority_after_worker_authentication() {
    for mutation in [
        ReleasedShutdownMutation::AuthToken,
        ReleasedShutdownMutation::SocketPath,
        ReleasedShutdownMutation::ControllerGeneration,
        ReleasedShutdownMutation::ExitEpoch,
    ] {
        for path in [ReleasedShutdownPath::Direct, ReleasedShutdownPath::Resume] {
            assert_released_shutdown_rejects_authenticated_replacement(mutation, path).await;
        }
    }
}

async fn assert_released_shutdown_rejects_authenticated_replacement(
    mutation: ReleasedShutdownMutation,
    path: ReleasedShutdownPath,
) {
    let (directory, host, record) = released_exit_fixture(ExitReason::Natural);
    let listener = UnixListener::bind(&record.socket_path).unwrap();
    let runtime_path = host.paths.runtime_record(record.terminal_id);
    let exit_path = host.paths.exit_record(record.terminal_id);
    let replacement_socket = directory.path().join(REPLACED_SHUTDOWN_SOCKET_NAME);
    let authenticated = record.clone();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream = UnixStream::from(stream);
        let auth: RequestEnvelope = read_frame(&mut stream).await.unwrap();
        replace_released_shutdown_evidence(
            mutation,
            &runtime_path,
            &exit_path,
            &authenticated,
            replacement_socket,
        );
        let response =
            ResponseEnvelope::success(&auth, &test_worker_hello(&authenticated)).unwrap();
        write_frame(&mut stream, &response).await.unwrap();
        let mut stream = yolo_runtime_rpc_testing::Peer::new(stream);
        match stream.request().await {
            Ok(request) => {
                let error = ErrorBody::internal(UNEXPECTED_SHUTDOWN_DETAIL);
                stream
                    .respond(&ResponseEnvelope::failure(&request, error))
                    .await
                    .unwrap();
                true
            }
            Err(_) => false,
        }
    });

    let error = match path {
        ReleasedShutdownPath::Direct => {
            let error = host
                .retire_released_worker_classified(&record)
                .await
                .unwrap_err();
            assert_eq!(
                error.observation(),
                WorkerRetirementObservation::Control {
                    phase: WorkerRetirementControlPhase::ReleasedEvidenceAfterAuth,
                    failure: WorkerRetirementControlFailure::RemoteRejected,
                }
            );
            error.into_error_body()
        }
        ReleasedShutdownPath::Resume => {
            let mut manifest = completed_test_manifest(&host, record.terminal_id);
            persist_host_session(&host, &manifest, &record);
            let error = host
                .settle_disconnected_previous_runtime(&mut manifest, &record)
                .await
                .unwrap_err()
                .into_explicit_error();
            assert_eq!(manifest.recovery_state, RecoveryState::Uncertain);
            error
        }
    };

    assert_eq!(error.code, crate::protocol::error_code::IDENTITY_MISMATCH);
    assert!(!error.retryable);
    assert!(!server.await.unwrap());
}

fn replace_released_shutdown_evidence(
    mutation: ReleasedShutdownMutation,
    runtime_path: &Path,
    exit_path: &Path,
    authenticated: &RuntimeRecord,
    replacement_socket: PathBuf,
) {
    if matches!(mutation, ReleasedShutdownMutation::ExitEpoch) {
        let mut exit = read_json::<ExitRecord>(exit_path).unwrap();
        exit.runtime_epoch = Uuid::new_v4();
        atomic_write_json(exit_path, &exit).unwrap();
        return;
    }
    let mut replacement = authenticated.clone();
    match mutation {
        ReleasedShutdownMutation::AuthToken => {
            replacement.auth_token = REPLACED_SHUTDOWN_AUTH_TOKEN.into();
        }
        ReleasedShutdownMutation::SocketPath => replacement.socket_path = replacement_socket,
        ReleasedShutdownMutation::ControllerGeneration => {
            replacement.controller_generation = replacement
                .controller_generation
                .saturating_add(SHUTDOWN_CONTROLLER_GENERATION_INCREMENT);
        }
        ReleasedShutdownMutation::ExitEpoch => unreachable!(),
    }
    atomic_write_json(runtime_path, &replacement).unwrap();
}
