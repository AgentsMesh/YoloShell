use crate::worker::tests::*;
const AUTH_IDENTITY_SOCKET_NAME: &str = "auth-identity.sock";
const AUTH_CONTROLLER_GENERATION_OFFSET: u64 = 1;
const CAPABILITY_AUTH_REQUEST_ID: &str = "capability-auth";
const UNAVAILABLE_CAPABILITY_HOLDER: &str = "unavailable-capability";
const UNAVAILABLE_CAPABILITY_INPUT: &[u8] = b"input-after-unavailable-termination\n";
const UNAVAILABLE_TERMINATION_ATTEMPTS: [bool; 4] = [false, true, false, true];

#[derive(Clone, Copy)]
enum AuthIdentityMismatch {
    Worker,
    RuntimeEpoch,
    Controller,
}

#[derive(Clone, Copy)]
enum TestOwnerCapability {
    Available,
    Unavailable,
}

#[tokio::test]
async fn worker_auth_identity_mismatches_are_non_retryable() {
    for mismatch in [
        AuthIdentityMismatch::Worker,
        AuthIdentityMismatch::RuntimeEpoch,
        AuthIdentityMismatch::Controller,
    ] {
        assert_auth_identity_mismatch(mismatch).await;
    }
}

async fn assert_auth_identity_mismatch(mismatch: AuthIdentityMismatch) {
    let directory = tempfile::tempdir().unwrap();
    let socket_path = directory.path().join(AUTH_IDENTITY_SOCKET_NAME);
    let listener = UnixListener::bind(&socket_path).unwrap();
    set_current_worker_socket_permissions(&socket_path).unwrap();
    let fixture = Fixture::new();
    let mut selected = lock_recover(&fixture.state.record).clone();
    selected.socket_path = socket_path;
    let mut reported = selected.clone();
    match mismatch {
        AuthIdentityMismatch::Worker => reported.worker_id = Uuid::new_v4(),
        AuthIdentityMismatch::RuntimeEpoch => reported.runtime_epoch = Uuid::new_v4(),
        AuthIdentityMismatch::Controller => {
            reported.controller_generation = selected
                .controller_generation
                .saturating_add(AUTH_CONTROLLER_GENERATION_OFFSET);
        }
    }
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let request: RequestEnvelope = read_frame(&mut stream).await.unwrap();
        let response = ResponseEnvelope::success(
            &request,
            &WorkerHello {
                runtime: reported,
                capabilities: Vec::new(),
            },
        )
        .unwrap();
        write_frame(&mut stream, &response).await.unwrap();
    });
    let generation = selected.controller_generation;
    let error = authenticated_stream(&selected, generation)
        .await
        .unwrap_err();
    assert!(matches!(&error, WorkerRpcError::IdentityMismatch(_)));
    let body = error.into_error_body();
    assert_eq!(body.code, crate::protocol::error_code::IDENTITY_MISMATCH);
    assert!(!body.retryable);
    server.await.unwrap();
}

include!("destructive_unavailable.rs");

#[test]
fn worker_auth_rejects_runtime_without_an_explicit_capability_declaration() {
    let fixture = Fixture::new();
    let runtime = lock_recover(&fixture.state.record).clone();
    assert!(from_value::<WorkerHello>(to_value(&runtime).unwrap()).is_err());
    let mut value = serde_json::to_value(WorkerHello {
        runtime,
        capabilities: Vec::new(),
    }).unwrap();
    value.as_object_mut().unwrap().remove(stringify!(capabilities));
    assert!(serde_json::from_value::<WorkerHello>(value).is_err());
}

#[tokio::test]
async fn worker_auth_reports_captured_destructive_capability() {
    let available = Fixture::new();
    let _available_commands =
        install_test_owner_with_capability(&available, TestOwnerCapability::Available);
    let hello = authenticate_fixture(&available).await;
    assert_eq!(
        hello.capabilities,
        vec![crate::protocol::capability::SAFE_PROCESS_TREE_TERMINATION.to_owned()]
    );

    let unavailable = Fixture::new();
    let _unavailable_commands =
        install_test_owner_with_capability(&unavailable, TestOwnerCapability::Unavailable);
    let hello = authenticate_fixture(&unavailable).await;
    assert!(hello.capabilities.is_empty());
}

fn install_test_owner(fixture: &Fixture) -> mpsc::Receiver<pty_owner_handle::PtyOwnerCommand> {
    install_test_owner_with_capability(fixture, TestOwnerCapability::Unavailable)
}

fn install_test_owner_with_capability(
    fixture: &Fixture,
    capability: TestOwnerCapability,
) -> mpsc::Receiver<pty_owner_handle::PtyOwnerCommand> {
    let (commands, receiver) = mpsc::channel();
    let settlement = crate::worker::pty_owner_handle::new_owner_settlement_for_test();
    let owner = match capability {
        TestOwnerCapability::Available => {
            pty_owner_handle::PtyOwnerHandle::for_test(commands, settlement)
        }
        TestOwnerCapability::Unavailable => {
            pty_owner_handle::PtyOwnerHandle::unavailable_for_test(commands, settlement)
        }
    };
    *lock_recover(&fixture.state.pty_owner) = Some(owner);
    receiver
}

async fn authenticate_fixture(fixture: &Fixture) -> WorkerHello {
    let (mut client, server) = UnixStream::pair().unwrap();
    let state = Arc::clone(&fixture.state);
    let task = tokio::spawn(async move { handle_connection(server, state).await });
    let runtime = lock_recover(&fixture.state.record).clone();
    let request = RequestEnvelope::new(
        CAPABILITY_AUTH_REQUEST_ID,
        WORKER_AUTH_METHOD,
        to_value(&WorkerAuth {
            token: runtime.auth_token,
            controller_generation: runtime.controller_generation,
        })
        .unwrap(),
    );
    write_frame(&mut client, &request).await.unwrap();
    let response: ResponseEnvelope = read_frame(&mut client).await.unwrap();
    let hello = response.decode_payload().unwrap();
    drop(client);
    task.await.unwrap().unwrap();
    hello
}
