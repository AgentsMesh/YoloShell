use super::super::*;
use super::server_support::*;
use super::support::*;
use super::support_operations::{OperationConnection, OperationFault};
use crate::protocol::{RequestEnvelope, ResponseEnvelope, method, read_frame, write_frame};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

const INCOMPATIBLE_PROTOCOL_VERSION_OFFSET: u16 = 1;

async fn assert_capacity_closes(socket: std::path::PathBuf) {
    let _blocker = UnixStream::connect(&socket).await.unwrap();
    tokio::task::yield_now().await;
    let mut rejected = UnixStream::connect(socket).await.unwrap();
    let mut byte = [0_u8; 1];
    assert_eq!(rejected.read(&mut byte).await.unwrap(), 0);
}

#[tokio::test]
async fn saturated_runtime_and_management_clients_are_closed_independently() {
    let connection = FakeConnection::new([]);
    let factory = FakeFactory::new(vec![connection], vec![ConnectStep::Connection(0)]);
    let harness = Harness::start_with_limits(
        factory,
        SATURATED_RUNTIME_CLIENT_LIMIT,
        SATURATED_MANAGEMENT_CLIENT_LIMIT,
    )
    .await;
    assert_capacity_closes(harness.paths.runtime_socket()).await;
    assert_capacity_closes(harness.paths.management_socket()).await;
    harness.stop().await;
}

#[tokio::test(start_paused = true)]
async fn silent_local_peers_are_closed_at_handshake_deadline() {
    let connection = FakeConnection::new([]);
    let factory = FakeFactory::new(vec![connection], vec![ConnectStep::Connection(0)]);
    let harness = Harness::start(factory).await;
    let runtime = UnixStream::connect(harness.paths.runtime_socket())
        .await
        .unwrap();
    let mut management = UnixStream::connect(harness.paths.management_socket())
        .await
        .unwrap();
    tokio::task::yield_now().await;
    tokio::time::advance(super::super::constants::LOCAL_HANDSHAKE_TIMEOUT).await;
    tokio::task::yield_now().await;
    let mut byte = [0_u8; 1];
    super::runtime_shutdown::assert_closed(runtime).await;
    assert_eq!(management.read(&mut byte).await.unwrap(), 0);
    harness.stop().await;
}

#[tokio::test(start_paused = true)]
async fn management_operation_deadline_cancels_unbounded_reconnect() {
    let factory = FakeFactory::empty(Vec::new());
    let harness = Harness::start(Arc::clone(&factory)).await;
    let socket = harness.paths.management_socket();
    let task = tokio::spawn(async move {
        BrokerClient::new(socket)
            .run_command(BrokerCommandRequest::standard(ALLOWED_PROGRAM, Vec::new()))
            .await
    });
    while factory.calls() == 0 {
        tokio::task::yield_now().await;
    }
    tokio::time::advance(super::super::timing::MANAGEMENT_OPERATION_TIMEOUT).await;
    assert!(task.await.unwrap().is_err());
    harness.stop().await;
}

#[tokio::test]
async fn invalid_protocol_and_transport_loss_receive_distinct_failures() {
    let connection = OperationConnection::new(OperationFault::CommandTransport);
    let factory = FakeFactory::new(vec![connection], vec![ConnectStep::Connection(0)]);
    let harness = Harness::start(factory).await;
    let mut stream = UnixStream::connect(harness.paths.management_socket())
        .await
        .unwrap();
    let mut envelope = BrokerRequestEnvelope::new(BrokerRequest::Status);
    envelope.protocol_version = super::super::constants::BROKER_PROTOCOL_VERSION
        .saturating_sub(INCOMPATIBLE_PROTOCOL_VERSION_OFFSET);
    write_frame(&mut stream, &envelope).await.unwrap();
    let response: BrokerResponse = read_frame(&mut stream).await.unwrap();
    assert_eq!(
        response.error_code.as_deref(),
        Some(super::super::constants::INVALID_REQUEST_ERROR_CODE)
    );

    let error = BrokerClient::new(harness.paths.management_socket())
        .run_command(BrokerCommandRequest::standard(ALLOWED_PROGRAM, Vec::new()))
        .await
        .err()
        .unwrap();
    assert!(matches!(
        error,
        BrokerClientError::Remote { code, .. }
            if code == super::super::constants::CONNECTION_LOST_ERROR_CODE
    ));
    harness.stop().await;

    let connection = OperationConnection::new(OperationFault::CommandRejected);
    let factory = FakeFactory::new(vec![connection], vec![ConnectStep::Connection(0)]);
    let harness = Harness::start(factory).await;
    let error = BrokerClient::new(harness.paths.management_socket())
        .run_command(BrokerCommandRequest::standard(ALLOWED_PROGRAM, Vec::new()))
        .await
        .err()
        .unwrap();
    assert!(matches!(
        error,
        BrokerClientError::Remote { code, .. }
            if code == super::super::constants::BROKER_ERROR_CODE
    ));
    harness.stop().await;
}

#[tokio::test]
async fn peer_eof_isolated_and_server_accessors_report_live_state() {
    let connection = FakeConnection::new([]);
    let factory = FakeFactory::new(vec![connection], vec![ConnectStep::Connection(0)]);
    let root = tempfile::tempdir().unwrap();
    let paths = BrokerPaths::new(root.path().join("state")).unwrap();
    let broker = RemoteBroker::with_factory(test_config(), paths.clone(), factory);
    assert_eq!(broker.paths().runtime_socket(), paths.runtime_socket());
    assert_eq!(broker.pool().status().await.host_id, HOST_ID);
    drop(broker);

    let connection = FakeConnection::new([]);
    let factory = FakeFactory::new(vec![connection], vec![ConnectStep::Connection(0)]);
    let harness = Harness::start(factory).await;
    drop(
        UnixStream::connect(harness.paths.runtime_socket())
            .await
            .unwrap(),
    );
    drop(
        UnixStream::connect(harness.paths.management_socket())
            .await
            .unwrap(),
    );
    tokio::task::yield_now().await;
    assert_eq!(
        BrokerClient::new(harness.paths.management_socket())
            .status()
            .await
            .unwrap()
            .host_id,
        HOST_ID
    );
    harness.stop().await;
}

#[tokio::test]
async fn stalled_remote_shutdowns_do_not_exhaust_runtime_capacity() {
    const CASES: usize = 3;
    let mut steps = vec![OpenStep::ShutdownStall; CASES];
    steps.push(OpenStep::Channel);
    let connection = FakeConnection::new(steps);
    let factory = FakeFactory::new(vec![connection.clone()], vec![ConnectStep::Connection(0)]);
    let harness = Harness::start_with_limits(
        factory,
        SATURATED_RUNTIME_CLIENT_LIMIT,
        SATURATED_MANAGEMENT_CLIENT_LIMIT,
    )
    .await;
    for opened in 1..=CASES {
        let client = UnixStream::connect(harness.paths.runtime_socket())
            .await
            .unwrap();
        let request = RequestEnvelope::new(
            PROXY_REQUEST_ID,
            method::HOST_STATUS,
            yolo_capnp_value::Value::Null,
        );
        let client = super::runtime_client::connect(client).await;
        assert!(super::runtime_client::call(&client, request).await.unwrap().ok);
        drop(client);
        connection.probe().wait_dropped(opened).await;
    }
    assert_eq!(
        proxy_request(harness.paths.runtime_socket(), method::HOST_STATUS)
            .await
            .connection_id,
        connection.id()
    );
    harness.stop().await;
}
