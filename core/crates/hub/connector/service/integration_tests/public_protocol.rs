use crate::fixture::*;
use tokio::{sync::oneshot, time::timeout_at};
use yolo_hub_connector_client::{Client, ClientError};
use yolo_hub_connector_protocol::{Command, ConnectorError, Reply};
use yolo_hub_connector_service::ConnectorServer;

#[tokio::test]
async fn real_private_socket_hello_list_and_anonymous_publish_need_no_host_process() {
    let fixture = Fixture::new();
    let server = ConnectorServer::bind(fixture.state.clone(), fixture.backend.clone())
        .await
        .unwrap();
    let socket = server.socket_path();
    let (stop, stopped) = oneshot::channel();
    let serving = tokio::spawn(server.serve_until(async {
        let _ = stopped.await;
    }));
    let mut client = Client::connect_local(&socket, fixture.state.hub_id(), deadline())
        .await
        .unwrap();
    assert_eq!(client.hub_id(), fixture.state.hub_id());
    assert_eq!(
        client
            .call(fixture.request(Command::List {}), deadline())
            .await
            .unwrap(),
        Reply::Enrollments {
            enrollments: vec![]
        }
    );
    let error = client
        .call(
            fixture.request(Command::Publish {
                publication: publication(fixture.activation()),
            }),
            deadline(),
        )
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        ClientError::Remote(ConnectorError::Unauthorized)
    ));
    assert!(!error.outcome_is_unknown());
    assert!(client.is_usable());
    assert_eq!(
        client
            .call(fixture.request(Command::Hello {}), deadline())
            .await
            .unwrap(),
        Reply::Hello {
            hub: fixture.state.hub_id()
        }
    );
    client.close(deadline()).await.unwrap();
    stop.send(()).unwrap();
    timeout_at(deadline(), serving)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(!socket.exists());
}

#[tokio::test]
async fn unavailable_registry_prevents_socket_publication() {
    let fixture = Fixture::new();
    let path = fixture.root.path().canonicalize().unwrap();
    let backend = yolo_hub_local_backend::LocalBackend::new(
        fixture.state.hub_id(),
        &path.join(crate::constants::HOST_DIRECTORY),
        &path.join(crate::constants::HOST_SOCKET),
    )
    .unwrap();
    let socket = fixture
        .state
        .socket_directory()
        .unwrap()
        .path()
        .join(yolo_hub_connector_service::CONNECTOR_SOCKET_FILE);
    assert!(
        ConnectorServer::bind(fixture.state.clone(), backend)
            .await
            .is_err()
    );
    assert!(!socket.exists());
}
