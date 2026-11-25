use super::*;
use yolo_hub_jsonrpc_protocol::Call as Request;
mod jsonrpc_ingress;
mod operation_deadlines;
use std::path::PathBuf;
use tokio::time::Instant;
use yolo_hub_identity::HubId;
use yolo_hub_jsonrpc_transport::Framed;
use yolo_hub_local_backend::LocalBackend;
use yolo_hub_rpc_protocol::Command;
use yolo_hub_state::HubState;
use yolo_local_unix::LocalUnixStream;

#[path = "../test_support/constants.rs"]
mod fixture_constants;
use fixture_constants::STATE_DIRECTORY;

const HOST_DIRECTORY: &str = "host";
const HOST_SOCKET: &str = "host.sock";
const SOCKET_TEST_ROOT: &str = "/tmp";

fn fixture() -> (tempfile::TempDir, HubState, LocalBackend) {
    let root = tempfile::tempdir_in(SOCKET_TEST_ROOT).unwrap();
    let path = root.path().canonicalize().unwrap();
    let home = path.join(HOST_DIRECTORY);
    let state = HubState::open(&path.join(STATE_DIRECTORY), &home).unwrap();
    let backend = LocalBackend::new(state.hub_id(), &home, &path.join(HOST_SOCKET)).unwrap();
    (root, state, backend)
}

#[tokio::test]
async fn binding_checks_identity_and_withdraws_only_its_socket() {
    let (root, state, _) = fixture();
    let path = root.path().canonicalize().unwrap();
    let invalid = LocalBackend::new(
        HubId::new(),
        &path.join(HOST_DIRECTORY),
        &path.join(HOST_SOCKET),
    )
    .unwrap();
    assert!(HubServer::bind(state, invalid).await.is_err());
    let state = HubState::open(&path.join(STATE_DIRECTORY), &path.join(HOST_DIRECTORY)).unwrap();
    let socket = state.socket_path();
    let backend = LocalBackend::new(
        state.hub_id(),
        &path.join(HOST_DIRECTORY),
        &path.join(HOST_SOCKET),
    )
    .unwrap();
    let server = HubServer::bind(state, backend).await.unwrap();
    assert!(socket.exists());
    server.serve_until(async {}).await.unwrap();
    assert!(!socket.exists());
}

#[tokio::test]
async fn missing_hello_is_rejected_before_contacting_the_host() {
    let (_root, state, backend) = fixture();
    let hub = state.hub_id();
    let (left, right) = LocalUnixStream::pair().unwrap();
    assert!(connection::same_user(&left));
    let server = connection::serve(
        right,
        std::sync::Arc::new(backend),
        std::sync::Arc::new(state),
    );
    let client = async {
        let mut channel = Framed::new(left);
        channel
            .send(
                &Request::new(
                    hub,
                    Command::List {
                        include_archived: false,
                        after: None,
                    },
                )
                .unwrap(),
                Instant::now() + OPERATION_TIMEOUT,
            )
            .await
            .unwrap();
        assert!(!matches!(
            channel
                .receive::<yolo_hub_jsonrpc_protocol::Answer>(Instant::now() + OPERATION_TIMEOUT)
                .await,
            Ok(yolo_hub_jsonrpc_transport::Incoming::Message(_))
        ));
    };
    let (result, _) = tokio::join!(server, client);
    assert!(result.is_err());
}

#[tokio::test]
async fn clean_eof_and_idle_shutdown_do_not_require_a_runtime() {
    let (_root, state, backend) = fixture();
    let (mut left, right) = LocalUnixStream::pair().unwrap();
    tokio::io::AsyncWriteExt::shutdown(&mut left).await.unwrap();
    assert!(
        connection::serve(
            right,
            std::sync::Arc::new(backend),
            std::sync::Arc::new(state)
        )
        .await
        .is_ok()
    );
    let (_root, state, backend) = fixture();
    let socket: PathBuf = state.socket_path();
    let server = HubServer::bind(state, backend).await.unwrap();
    let (stop, stopped) = tokio::sync::oneshot::channel();
    let serving = tokio::spawn(server.serve_until(async {
        let _ = stopped.await;
    }));
    let client = LocalUnixStream::connect(&socket).await.unwrap();
    stop.send(()).unwrap();
    serving.await.unwrap().unwrap();
    drop(client);
    assert!(!socket.exists());
}
