use super::super::handlers::{manage, proxy_runtime};
use super::super::server_admission::BrokerAdmission;
use super::super::*;
use super::support::*;
use crate::protocol::{RequestEnvelope, method, read_frame, write_frame};
use std::sync::Arc;

const EXPECTED_BOOTSTRAP_CONNECTIONS: usize = 1;
const TEST_CONNECTION_INDEX: usize = 0;

#[tokio::test]
async fn completed_runtime_proxy_shuts_down_remote_channel_cleanly() {
    let connection = FakeConnection::new([]);
    let factory = FakeFactory::new(
        vec![connection],
        vec![ConnectStep::Connection(TEST_CONNECTION_INDEX)],
    );
    let config = test_config();
    let pool = Arc::new(BrokerConnectionPool::new(
        HOST_ID,
        config.revision(),
        config.runtime().clone(),
        factory,
    ));
    let (client, server) = tokio::net::UnixStream::pair().unwrap();
    let task = tokio::spawn(proxy_runtime(server.into(), pool));
    let client = super::runtime_client::connect(client).await;
    let request = RequestEnvelope::new(
        PROXY_REQUEST_ID,
        method::HOST_STATUS,
        yolo_capnp_value::Value::Null,
    );
    let response = super::runtime_client::call(&client, request).await.unwrap();
    assert!(response.ok);
    drop(client);
    let result = tokio::time::timeout(super::super::constants::LOCAL_HANDSHAKE_TIMEOUT, task)
        .await
        .unwrap()
        .unwrap();
    result.unwrap();
}

#[tokio::test]
async fn pending_broker_bootstraps_but_rejects_other_management_work() {
    let connection = FakeConnection::new([]);
    let factory = FakeFactory::new(
        vec![connection],
        vec![ConnectStep::Connection(TEST_CONNECTION_INDEX)],
    );
    let config = Arc::new(test_config());
    let pool = Arc::new(BrokerConnectionPool::new(
        HOST_ID,
        config.revision(),
        config.runtime().clone(),
        factory.clone(),
    ));
    let (mut client, server) = tokio::net::UnixStream::pair().unwrap();
    let task = tokio::spawn(manage(
        server.into(),
        pool,
        config,
        Arc::new(tokio::sync::Notify::new()),
        Arc::new(tokio::sync::Notify::new()),
        Arc::new(BrokerAdmission::new(true)),
    ));

    let bootstrap = BrokerRequestEnvelope::new(BrokerRequest::Bootstrap);
    write_frame(&mut client, &bootstrap).await.unwrap();
    let response: BrokerResponse = read_frame(&mut client).await.unwrap();
    let Some(BrokerResult::Bootstrap(status)) = response.result else {
        panic!("pending broker did not return bootstrap status")
    };
    assert!(!status.adopted);
    assert!(status.connection_id.is_some());

    let working = BrokerRequestEnvelope::new(BrokerRequest::WorkingDirectory);
    write_frame(&mut client, &working).await.unwrap();
    let response: BrokerResponse = read_frame(&mut client).await.unwrap();
    assert!(response.result.is_none());
    assert_eq!(
        response.error_message.as_deref(),
        Some(super::super::constants::BROKER_ADOPTION_PENDING_MESSAGE)
    );
    assert_eq!(factory.calls(), EXPECTED_BOOTSTRAP_CONNECTIONS);
    task.abort();
}
