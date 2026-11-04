use super::super::*;
use super::server_support::*;
use super::support::*;
use crate::Client;
use crate::protocol::method;
use std::sync::Arc;
use tokio::net::UnixStream;

const CANCELLATION_SETTLE_DELAY: std::time::Duration = std::time::Duration::from_millis(250);
const CANCELLATION_OBSERVATION_WINDOW: std::time::Duration = std::time::Duration::from_millis(350);

#[tokio::test]
async fn runtime_and_management_capacity_are_independent_fault_domains() {
    let connection = FakeConnection::new([]);
    let factory = FakeFactory::new(vec![connection.clone()], vec![ConnectStep::Connection(0)]);
    let harness = Harness::start_with_limits(
        factory,
        SATURATED_RUNTIME_CLIENT_LIMIT,
        SATURATED_MANAGEMENT_CLIENT_LIMIT,
    )
    .await;
    let management_blocker = UnixStream::connect(harness.paths.management_socket())
        .await
        .unwrap();
    tokio::task::yield_now().await;
    let reply = proxy_request(harness.paths.runtime_socket(), method::HOST_STATUS).await;
    assert_eq!(reply.connection_id, connection.id());
    drop(management_blocker);
    harness.stop().await;

    let connection = FakeConnection::new([]);
    let factory = FakeFactory::new(vec![connection], vec![ConnectStep::Connection(0)]);
    let harness = Harness::start_with_limits(
        factory,
        SATURATED_RUNTIME_CLIENT_LIMIT,
        SATURATED_MANAGEMENT_CLIENT_LIMIT,
    )
    .await;
    let runtime_blocker = UnixStream::connect(harness.paths.runtime_socket())
        .await
        .unwrap();
    tokio::task::yield_now().await;
    let status = BrokerClient::new(harness.paths.management_socket())
        .status()
        .await
        .unwrap();
    assert_eq!(status.host_id, HOST_ID);
    drop(runtime_blocker);
    harness.stop().await;
}

#[tokio::test]
async fn concurrent_control_and_attachment_proxies_share_connection_id() {
    let connection = FakeConnection::new([]);
    let factory = FakeFactory::new(
        vec![Arc::clone(&connection)],
        vec![ConnectStep::Connection(0)],
    );
    let harness = Harness::start(Arc::clone(&factory)).await;
    let mut requests = Vec::new();
    for index in 0..20 {
        let socket = harness.paths.runtime_socket();
        requests.push(tokio::spawn(async move {
            let method_name = if index % 2 == 0 {
                method::ATTACH_SESSION
            } else {
                method::HOST_STATUS
            };
            proxy_request(socket, method_name).await
        }));
    }
    for request in requests {
        assert_eq!(request.await.unwrap().connection_id, connection.id());
    }
    assert_eq!(factory.calls(), 1);
    harness.stop().await;
}

#[tokio::test]
async fn failed_channel_does_not_break_an_existing_proxy() {
    let connection = FakeConnection::new([OpenStep::Channel, OpenStep::Rejected]);
    let factory = FakeFactory::new(vec![connection.clone()], vec![ConnectStep::Connection(0)]);
    let harness = Harness::start(factory).await;
    let client = Client::new(harness.paths.runtime_socket());
    let mut active = client.connect().await.unwrap();
    let first: ProxyReply = active.request(method::HOST_STATUS, &()).await.unwrap();
    assert_eq!(first.connection_id, connection.id());
    assert!(client.raw(method::HOST_STATUS, &()).await.is_err());
    let second: ProxyReply = active.request(method::HOST_STATUS, &()).await.unwrap();
    assert_eq!(second.connection_id, connection.id());
    harness.stop().await;
}

#[tokio::test]
async fn transport_reconnect_keeps_old_remote_channel_alive() {
    let first = FakeConnection::new([OpenStep::Channel, OpenStep::TransportFailure]);
    let second = FakeConnection::new([]);
    let factory = FakeFactory::new(
        vec![first.clone(), second.clone()],
        vec![ConnectStep::Connection(0), ConnectStep::Connection(1)],
    );
    let harness = Harness::start(factory).await;
    let client = Client::new(harness.paths.runtime_socket());
    let mut old = client.connect().await.unwrap();
    let old_reply: ProxyReply = old.request(method::HOST_STATUS, &()).await.unwrap();
    assert_eq!(old_reply.connection_id, first.id());
    let new_reply = proxy_request(harness.paths.runtime_socket(), method::HOST_STATUS).await;
    assert_eq!(new_reply.connection_id, second.id());
    let still_alive: ProxyReply = old.request(method::HOST_STATUS, &()).await.unwrap();
    assert_eq!(still_alive.connection_id, first.id());
    harness.stop().await;
}

#[tokio::test]
async fn management_command_file_and_attachment_are_parallel() {
    let connection = FakeConnection::new([]);
    let factory = FakeFactory::new(vec![connection.clone()], vec![ConnectStep::Connection(0)]);
    let harness = Harness::start(factory).await;
    let management = BrokerClient::new(harness.paths.management_socket());
    let command = BrokerCommandRequest::standard(ALLOWED_PROGRAM, vec!["status".into()]);
    let file = BrokerFileRequest {
        path: READ_PATH.into(),
        offset: 0,
        length: 4,
    };
    let (command, file, attachment) = tokio::join!(
        management.run_command(command),
        management.read_file(file),
        proxy_request(harness.paths.runtime_socket(), method::ATTACH_SESSION),
    );
    assert!(
        matches!(command.unwrap(), BrokerResult::Command { connection_id, .. } if connection_id == connection.id())
    );
    assert!(
        matches!(file.unwrap(), BrokerResult::FileRead { connection_id, .. } if connection_id == connection.id())
    );
    assert_eq!(attachment.connection_id, connection.id());
    harness.stop().await;
}

#[tokio::test]
async fn disconnected_local_peer_cancels_unbounded_reconnect() {
    let factory = FakeFactory::empty(vec![]);
    let harness = Harness::start(Arc::clone(&factory)).await;
    let socket = harness.paths.runtime_socket();
    let request =
        tokio::spawn(async move { Client::new(socket).raw(method::HOST_STATUS, &()).await });
    while factory.calls() < 2 {
        tokio::task::yield_now().await;
    }
    request.abort();
    tokio::time::sleep(CANCELLATION_SETTLE_DELAY).await;
    let settled = factory.calls();
    tokio::time::sleep(CANCELLATION_OBSERVATION_WINDOW).await;
    assert_eq!(factory.calls(), settled);
    harness.stop().await;
}

#[tokio::test(start_paused = true)]
async fn local_eof_bounds_a_stalled_runtime_probe_write() {
    let stalled_connection = FakeConnection::new([OpenStep::StalledChannel, OpenStep::Rejected]);
    let replacement_connection = FakeConnection::new([OpenStep::Channel]);
    let factory = FakeFactory::new(
        vec![stalled_connection.clone(), replacement_connection.clone()],
        vec![ConnectStep::Connection(0), ConnectStep::Connection(1)],
    );
    let harness = Harness::start_with_limits(
        factory,
        SATURATED_RUNTIME_CLIENT_LIMIT,
        SATURATED_MANAGEMENT_CLIENT_LIMIT,
    )
    .await;
    let socket = harness.paths.runtime_socket();
    let stalled =
        tokio::spawn(async move { Client::new(socket).raw(method::HOST_STATUS, &()).await });
    while stalled_connection.opens().is_empty() {
        tokio::task::yield_now().await;
    }
    stalled.abort();
    let _ = stalled.await;
    stalled_connection.probe().wait_write_pending(1).await;
    tokio::time::advance(super::super::constants::LOCAL_HANDSHAKE_TIMEOUT).await;
    stalled_connection.probe().wait_dropped(1).await;
    let reply = proxy_request(harness.paths.runtime_socket(), method::HOST_STATUS).await;
    assert_eq!(reply.connection_id, replacement_connection.id());
    harness.stop().await;
}
