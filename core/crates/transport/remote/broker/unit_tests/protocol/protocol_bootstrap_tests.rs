use super::*;
use crate::remote_broker::client_io::{ConnectedBroker, bootstrap_on};
use crate::remote_broker::timing::BROKER_CLIENT_OPERATION_TIMEOUT;

#[tokio::test]
async fn client_bootstrap_returns_the_connected_broker_identity() {
    let server = Server::start(ReplyKind::Bootstrap).await;

    let status = BrokerClient::new(&server.path).bootstrap().await.unwrap();

    assert_eq!(status.host_id, HOST_ID);
    assert_eq!(status.connection_generation, CONNECTION_GENERATION);
    server.task.await.unwrap();
}

#[tokio::test]
async fn bootstrap_at_obeys_an_expired_caller_deadline() {
    let server = Server::start(ReplyKind::Bootstrap).await;
    let identity = crate::remote_broker::socket_identity::socket_identity(&server.path).unwrap();

    let result = BrokerClient::new(&server.path)
        .bootstrap_at(identity, tokio::time::Instant::now())
        .await;

    assert!(matches!(result, Err(BrokerClientError::Deadline)));
    server.task.abort();
}

#[tokio::test]
async fn path_verification_rejects_a_deadline_expired_after_the_response() {
    let server = Server::start(ReplyKind::Bootstrap).await;
    let identity = crate::remote_broker::socket_identity::socket_identity(&server.path).unwrap();
    let deadline = tokio::time::Instant::now() + BROKER_CLIENT_OPERATION_TIMEOUT;
    let mut connection = ConnectedBroker::connect(server.path.clone(), Some(identity), deadline)
        .await
        .unwrap();

    let status = bootstrap_on(connection.stream(), deadline).await.unwrap();
    assert_eq!(status.host_id, HOST_ID);
    tokio::time::pause();
    tokio::time::advance(deadline.saturating_duration_since(tokio::time::Instant::now())).await;

    assert!(matches!(
        connection.verify_path(),
        Err(BrokerClientError::Deadline)
    ));
    server.task.await.unwrap();
}
