use super::super::*;
use super::server_support::*;
use super::support::*;
use crate::Client;
use crate::protocol::{RequestEnvelope, method};
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::net::UnixStream;

const WRONG_REVISION: &str = "wrong-config-revision";
const INSTANCE_CHANGED_MESSAGE: &str = "broker instance changed";
const CLOSED_STREAM_PROBE_BYTES: usize = 1;
const PENDING_OPEN_REQUEST_ID: &str = "pending-open-during-rotation";

#[tokio::test]
async fn shutdown_rejects_revision_mismatch_without_stopping_broker() {
    let connection = FakeConnection::new([]);
    let factory = FakeFactory::new(vec![connection], vec![ConnectStep::Connection(0)]);
    let harness = Harness::start(factory).await;
    let client = BrokerClient::new(harness.paths.management_socket());
    assert!(client.shutdown(WRONG_REVISION.into()).await.is_err());
    assert_eq!(
        client.status().await.unwrap().config_revision,
        CONFIG_REVISION
    );
    harness.stop().await;
}

#[tokio::test]
async fn shutdown_rejects_instance_mismatch_without_stopping_broker() {
    let factory = FakeFactory::empty(Vec::new());
    let harness = Harness::start(factory).await;
    let client = BrokerClient::new(harness.paths.management_socket());
    let status = client.status().await.unwrap();
    let error = client
        .request(BrokerRequest::Shutdown {
            expected_revision: status.config_revision.clone(),
            expected_instance_id: uuid::Uuid::nil(),
        })
        .await
        .unwrap_err();

    assert!(error.to_string().contains(INSTANCE_CHANGED_MESSAGE));
    assert_eq!(
        client.status().await.unwrap().instance_id,
        status.instance_id
    );
    harness.stop().await;
}

#[tokio::test]
async fn shutdown_response_arrives_before_accept_loop_stops() {
    let connection = FakeConnection::new([]);
    let factory = FakeFactory::new(vec![connection], vec![ConnectStep::Connection(0)]);
    let harness = Harness::start(factory).await;
    BrokerClient::new(harness.paths.management_socket())
        .shutdown(CONFIG_REVISION.into())
        .await
        .unwrap();
    harness.wait_stopped().await;
}

#[tokio::test]
async fn graceful_shutdown_keeps_existing_runtime_proxy_alive() {
    let connection = FakeConnection::new([]);
    let factory = FakeFactory::new(vec![connection.clone()], vec![ConnectStep::Connection(0)]);
    let harness = Harness::start(factory).await;
    let mut active = Client::new(harness.paths.runtime_socket())
        .connect()
        .await
        .unwrap();
    let first: ProxyReply = active.request(method::HOST_STATUS, &()).await.unwrap();
    assert_eq!(first.connection_id, connection.id());
    BrokerClient::new(harness.paths.management_socket())
        .shutdown(CONFIG_REVISION.into())
        .await
        .unwrap();
    harness.wait_socket_removed().await;
    let second: ProxyReply = active.request(method::HOST_STATUS, &()).await.unwrap();
    assert_eq!(second.connection_id, connection.id());
    drop(active);
    harness.wait_stopped().await;
}

#[tokio::test(start_paused = true)]
async fn rotation_cancels_management_but_naturally_drains_runtime() {
    let connection = FakeConnection::new([]);
    let factory = FakeFactory::new(vec![connection.clone()], vec![ConnectStep::Connection(0)]);
    let harness = Harness::start(factory).await;
    let _clock = super::runtime_client::DrivenClock::new();
    let mut idle_management = UnixStream::connect(harness.paths.management_socket())
        .await
        .unwrap();
    let mut active = Client::new(harness.paths.runtime_socket())
        .connect()
        .await
        .unwrap();
    let _: ProxyReply = active.request(method::HOST_STATUS, &()).await.unwrap();
    BrokerClient::new(harness.paths.management_socket())
        .shutdown(CONFIG_REVISION.into())
        .await
        .unwrap();
    harness.wait_socket_removed().await;
    tokio::time::advance(super::super::constants::MANAGEMENT_DRAIN_TIMEOUT).await;
    tokio::task::yield_now().await;
    let mut probe = [0_u8; CLOSED_STREAM_PROBE_BYTES];
    assert_eq!(idle_management.read(&mut probe).await.unwrap(), 0);
    assert!(!harness.task_finished());
    let reply: ProxyReply = active.request(method::HOST_STATUS, &()).await.unwrap();
    assert_eq!(reply.connection_id, connection.id());
    drop(active);
    harness.wait_stopped().await;
}

#[tokio::test(start_paused = true)]
async fn external_shutdown_bounds_all_active_client_tasks() {
    let connection = FakeConnection::new([]);
    let factory = FakeFactory::new(vec![connection], vec![ConnectStep::Connection(0)]);
    let harness = Harness::start(factory).await;
    let _clock = super::runtime_client::DrivenClock::new();
    let runtime = UnixStream::connect(harness.paths.runtime_socket())
        .await
        .unwrap();
    let mut management = UnixStream::connect(harness.paths.management_socket())
        .await
        .unwrap();
    let stopped = tokio::spawn(harness.stop());
    tokio::task::yield_now().await;
    assert!(!stopped.is_finished());
    tokio::time::advance(super::super::constants::EXTERNAL_RUNTIME_DRAIN_TIMEOUT).await;
    tokio::task::yield_now().await;
    drop(_clock);
    stopped.await.unwrap();
    let mut probe = [0_u8; CLOSED_STREAM_PROBE_BYTES];
    super::runtime_shutdown::assert_closed(runtime).await;
    assert_eq!(management.read(&mut probe).await.unwrap(), 0);
}

#[tokio::test(start_paused = true)]
async fn rotation_cancels_runtime_that_has_not_opened_a_remote_channel() {
    let factory = FakeFactory::empty(vec![]);
    let harness = Harness::start(Arc::clone(&factory)).await;
    let _clock = super::runtime_client::DrivenClock::new();
    let pending = UnixStream::connect(harness.paths.runtime_socket())
        .await
        .unwrap();
    let pending = super::runtime_client::connect(pending).await;
    let request = super::runtime_client::pending(&pending, RequestEnvelope::new(
            PENDING_OPEN_REQUEST_ID,
            method::HOST_STATUS,
            yolo_capnp_value::Value::Null,
        ));
    while factory.calls() == 0 {
        tokio::task::yield_now().await;
    }
    BrokerClient::new(harness.paths.management_socket())
        .shutdown(CONFIG_REVISION.into())
        .await
        .unwrap();
    harness.wait_socket_removed().await;
    tokio::time::advance(super::super::constants::RUNTIME_SETUP_DRAIN_TIMEOUT).await;
    drop(_clock);
    harness.wait_stopped().await;
    assert!(request.await.unwrap().is_err());
    drop(pending);
}

#[tokio::test(start_paused = true)]
async fn external_shutdown_escalates_an_already_rotating_broker() {
    let connection = FakeConnection::new([]);
    let factory = FakeFactory::new(vec![connection], vec![ConnectStep::Connection(0)]);
    let harness = Harness::start(factory).await;
    let _clock = super::runtime_client::DrivenClock::new();
    let mut active = Client::new(harness.paths.runtime_socket())
        .connect()
        .await
        .unwrap();
    let _: ProxyReply = active.request(method::HOST_STATUS, &()).await.unwrap();
    BrokerClient::new(harness.paths.management_socket())
        .shutdown(CONFIG_REVISION.into())
        .await
        .unwrap();
    harness.wait_socket_removed().await;
    let stopped = tokio::spawn(harness.stop());
    tokio::task::yield_now().await;
    assert!(!stopped.is_finished());
    tokio::time::advance(super::super::constants::EXTERNAL_RUNTIME_DRAIN_TIMEOUT).await;
    drop(_clock);
    stopped.await.unwrap();
    let closed: Result<ProxyReply, _> = active.request(method::HOST_STATUS, &()).await;
    assert!(closed.is_err());
}
