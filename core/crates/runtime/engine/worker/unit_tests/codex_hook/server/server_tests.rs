use super::super::constants::CODEX_ACK_PROTOCOL;
use super::super::ingress_supervisor::CodexHookIngressSupervisor;
use super::ingress_test_support::IngressEndpoint;
use super::native_ingress_support::*;
use crate::local_unix::LocalUnixStream;
use std::sync::{Arc, Mutex, MutexGuard};

const MALFORMED_SERVER_REQUEST: &[u8] = b"{";
const RESTART_HOME_DIRECTORY: &str = "restart-home";
const REPLAY_HOME_DIRECTORY: &str = "replay-home";
const PROTOCOL_HOME_DIRECTORY: &str = "protocol-home";
static SERVER_TEST_SERIALIZATION: Mutex<()> = Mutex::new(());

fn serialize_server_test() -> MutexGuard<'static, ()> {
    SERVER_TEST_SERIALIZATION
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[tokio::test]
async fn retained_server_restart_preserves_endpoint_and_committed_route() {
    let _serialization = serialize_server_test();
    let fixture = Fixture::new();
    let endpoint = IngressEndpoint::new(RESTART_HOME_DIRECTORY);
    let supervisor = CodexHookIngressSupervisor::new();
    let mut server = endpoint.bind_server(&supervisor, &fixture).await;
    let socket = server.socket_path().to_path_buf();
    let runtime = endpoint.bound_runtime(&server, &fixture).await;

    assert!(socket.exists());
    server.restart_retained().await.unwrap();
    assert_eq!(server.socket_path(), socket);
    let replayed = endpoint.bound_runtime(&server, &fixture).await;
    assert!(Arc::ptr_eq(&runtime, &replayed));

    drop((server, runtime, replayed));
    endpoint.finish();
}

#[tokio::test]
async fn native_registration_replays_first_event_before_committing_route() {
    let _serialization = serialize_server_test();
    let fixture = Fixture::new();
    let endpoint = IngressEndpoint::new(REPLAY_HOME_DIRECTORY);
    let supervisor = CodexHookIngressSupervisor::new();
    let server = endpoint.bind_server(&supervisor, &fixture).await;

    let runtime = endpoint.bound_runtime(&server, &fixture).await;

    assert_eq!(
        runtime.state().unwrap().unwrap().last_sequence.0,
        crate::agent::hooks::FIRST_SEQUENCE
    );
    drop((server, runtime));
    endpoint.finish();
}

#[tokio::test]
async fn retained_server_contains_malformed_client_request() {
    let _serialization = serialize_server_test();
    let fixture = Fixture::new();
    let endpoint = IngressEndpoint::new(PROTOCOL_HOME_DIRECTORY);
    let supervisor = CodexHookIngressSupervisor::new();
    let server = endpoint.bind_server(&supervisor, &fixture).await;
    let connected = LocalUnixStream::connect(server.socket_path());
    let accepted = server.accept();
    let (client, accepted) = tokio::join!(connected, accepted);
    let mut client = client.unwrap();
    let accepted = accepted.unwrap().0;
    let exchange = async {
        write_bytes_frame(&mut client, MALFORMED_SERVER_REQUEST).await;
        read_ack(&mut client).await
    };

    let (publication, ack) = tokio::join!(server.handle_stream(accepted), exchange);

    let (publication, response) = publication.unwrap().into_parts();
    response.unwrap();
    assert!(publication.is_none());
    assert!(!ack.accepted);
    assert_eq!(ack.code, CODEX_ACK_PROTOCOL);
    drop(server);
    endpoint.finish();
}
