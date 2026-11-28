use crate::{constants::*, fixture::*};
use tokio::{
    sync::oneshot,
    time::{sleep, timeout_at},
};
use yolo_hub_connector_client::Client;
use yolo_hub_connector_protocol::Command;
use yolo_hub_connector_service::{CONNECTOR_SOCKET_FILE, ConnectorServer};
use yolo_local_unix::LocalUnixStream;

#[tokio::test]
async fn bind_rejects_wrong_backend_before_publishing_and_keeps_single_listener_owner() {
    let fixture = Fixture::new();
    let socket = fixture
        .state
        .socket_directory()
        .unwrap()
        .path()
        .join(CONNECTOR_SOCKET_FILE);
    assert!(
        ConnectorServer::bind(fixture.state.clone(), fixture.invalid_backend())
            .await
            .is_err()
    );
    assert!(!socket.exists());
    let server = ConnectorServer::bind(fixture.state.clone(), fixture.backend.clone())
        .await
        .unwrap();
    assert_eq!(server.socket_path(), socket);
    assert!(
        ConnectorServer::bind(fixture.state.clone(), fixture.backend.clone())
            .await
            .is_err()
    );
    assert!(socket.exists());
    server.serve_until(async {}).await.unwrap();
    assert!(!socket.exists());
    fixture.state.verify().unwrap();
}

#[tokio::test]
async fn normal_shutdown_aborts_idle_channels_and_withdraws_only_the_connector_socket() {
    let fixture = Fixture::new();
    let server = ConnectorServer::bind(fixture.state.clone(), fixture.backend.clone())
        .await
        .unwrap();
    let socket = server.socket_path();
    let (stop, stopped) = oneshot::channel();
    let serving = tokio::spawn(server.serve_until(async {
        let _ = stopped.await;
    }));
    let idle = LocalUnixStream::connect(&socket).await.unwrap();
    let mut client = Client::connect_local(&socket, fixture.state.hub_id(), deadline())
        .await
        .unwrap();
    stop.send(()).unwrap();
    timeout_at(deadline(), serving)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(!socket.exists());
    assert!(fixture.backend.list_connectors().await.unwrap().is_empty());
    assert!(
        client
            .call(fixture.request(Command::Hello {}), deadline())
            .await
            .is_err()
    );
    drop(idle);
    fixture.state.verify().unwrap();
}

#[tokio::test]
async fn cancelling_outer_serve_future_retains_cleanup_until_socket_is_withdrawn() {
    let fixture = Fixture::new();
    let server = ConnectorServer::bind(fixture.state.clone(), fixture.backend.clone())
        .await
        .unwrap();
    let socket = server.socket_path();
    let serving = tokio::spawn(server.serve_until(std::future::pending()));
    let mut client = Client::connect_local(&socket, fixture.state.hub_id(), deadline())
        .await
        .unwrap();
    serving.abort();
    assert!(serving.await.unwrap_err().is_cancelled());
    timeout_at(deadline(), async {
        while socket.exists() {
            sleep(POLL_INTERVAL).await;
        }
    })
    .await
    .unwrap();
    assert!(
        client
            .call(fixture.request(Command::Hello {}), deadline())
            .await
            .is_err()
    );
    assert!(fixture.backend.list_connectors().await.unwrap().is_empty());
    fixture.state.verify().unwrap();
    let replacement = ConnectorServer::bind(fixture.state.clone(), fixture.backend.clone())
        .await
        .unwrap();
    replacement.serve_until(async {}).await.unwrap();
    assert!(!socket.exists());
}

#[tokio::test]
async fn registry_cleanup_failure_is_reported_and_does_not_skip_socket_withdrawal() {
    let fixture = Fixture::new();
    let server = ConnectorServer::bind(fixture.state.clone(), fixture.backend.clone())
        .await
        .unwrap();
    let socket = server.socket_path();
    std::fs::write(fixture.connector_path.join(INDEX_FILE), CORRUPT_BYTES).unwrap();
    let result = server.serve_until(async {}).await;
    assert!(result.is_err());
    assert!(!socket.exists());
    assert!(fixture.backend.list_connectors().await.is_err());
}

#[tokio::test]
async fn dropping_a_bound_unstarted_service_retires_publication_without_spawning_work() {
    let fixture = Fixture::new();
    let server = ConnectorServer::bind(fixture.state.clone(), fixture.backend.clone())
        .await
        .unwrap();
    let socket = server.socket_path();
    assert!(socket.exists());
    drop(server);
    assert!(!socket.exists());
    fixture.state.verify().unwrap();
}
