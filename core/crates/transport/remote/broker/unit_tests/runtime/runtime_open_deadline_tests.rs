use super::super::handlers::proxy_runtime_admitted;
use super::super::*;
use super::support::*;
use crate::protocol::{RequestEnvelope, method};
use crate::remote::{RemoteError, RuntimeRole};
use std::sync::Arc;
use tokio::sync::Semaphore;

const AVAILABLE_ATTACHMENT_PERMITS: usize = 1;
const FIRST_CONNECTION_INDEX: usize = 0;
const NO_AVAILABLE_ATTACHMENT_PERMITS: usize = 0;
const EXPECTED_DROPPED_CHANNELS: usize = 1;
const EXPECTED_PENDING_WRITES: usize = 1;

fn pool(factory: Arc<dyn BrokerConnectionFactory>) -> Arc<BrokerConnectionPool> {
    let config = test_config();
    Arc::new(BrokerConnectionPool::new(
        HOST_ID,
        config.revision(),
        config.runtime().clone(),
        factory,
    ))
}

fn assert_timeout(error: &RemoteError) {
    assert!(matches!(
        error,
        RemoteError::Transport(message) if message == crate::remote::OPERATION_TIMEOUT_MESSAGE
    ));
}

async fn expire_open(
    task: tokio::task::JoinHandle<Result<yolo_runtime_rpc::RuntimeConnection, RemoteError>>,
) {
    tokio::time::advance(crate::remote::SSH_OPERATION_TIMEOUT).await;
    assert_timeout(&task.await.unwrap().err().unwrap());
}

#[tokio::test(start_paused = true)]
async fn runtime_open_deadline_cancels_a_stalled_channel_open() {
    let _clock = super::runtime_client::DrivenClock::new();
    let connection = FakeConnection::new([OpenStep::BlockedOpen]);
    let task_pool = pool(FakeFactory::new(
        vec![connection.clone()],
        vec![ConnectStep::Connection(FIRST_CONNECTION_INDEX)],
    ));
    let task = tokio::spawn(async move { task_pool.open_runtime(RuntimeRole::Control).await });
    while connection.opens().is_empty() {
        tokio::task::yield_now().await;
    }

    expire_open(task).await;
}

#[tokio::test(start_paused = true)]
async fn runtime_open_deadline_drops_a_channel_stalled_during_preflight() {
    let _clock = super::runtime_client::DrivenClock::new();
    let connection = FakeConnection::new([OpenStep::GatedHandshakeWrite, OpenStep::Channel]);
    let factory = FakeFactory::new(
        vec![connection.clone()],
        vec![
            ConnectStep::Connection(FIRST_CONNECTION_INDEX),
            ConnectStep::Connection(FIRST_CONNECTION_INDEX),
        ],
    );
    let shared = pool(factory);
    let task_pool = Arc::clone(&shared);
    let task = tokio::spawn(async move { task_pool.open_runtime(RuntimeRole::Control).await });
    connection
        .probe()
        .wait_write_pending(EXPECTED_PENDING_WRITES)
        .await;

    expire_open(task).await;
    connection
        .probe()
        .wait_dropped(EXPECTED_DROPPED_CHANNELS)
        .await;
    assert!(shared.status().await.connection_id.is_none());
    drop(shared.open_runtime(RuntimeRole::Control).await.unwrap());
    assert!(shared.status().await.connection_id.is_some());
}

#[tokio::test(start_paused = true)]
async fn attachment_permit_is_released_after_runtime_open_timeout() {
    let _clock = super::runtime_client::DrivenClock::new();
    let connection = FakeConnection::new([OpenStep::BlockedOpen]);
    let shared = pool(FakeFactory::new(
        vec![connection.clone()],
        vec![ConnectStep::Connection(FIRST_CONNECTION_INDEX)],
    ));
    let attachments = Arc::new(Semaphore::new(AVAILABLE_ATTACHMENT_PERMITS));
    let (client, server) = tokio::net::UnixStream::pair().unwrap();
    let proxy = tokio::spawn(proxy_runtime_admitted(
        server.into(),
        shared,
        Arc::clone(&attachments),
    ));
    let request = RequestEnvelope::new(
        PROXY_REQUEST_ID,
        method::ATTACH_SESSION,
        yolo_capnp_value::Value::Null,
    );
    let client = super::runtime_client::connect(client).await;
    let pending = super::runtime_client::pending(&client, request);
    while connection.opens().is_empty() {
        tokio::task::yield_now().await;
    }
    assert_eq!(
        attachments.available_permits(),
        NO_AVAILABLE_ATTACHMENT_PERMITS
    );

    tokio::time::advance(crate::remote::SSH_OPERATION_TIMEOUT).await;
    let error = pending.await.unwrap().unwrap_err();
    assert!(matches!(error, yolo_capnp_rpc::Error::OutcomeUnknown));
    assert_eq!(
        attachments.available_permits(),
        AVAILABLE_ATTACHMENT_PERMITS
    );
    drop(client);
    proxy.await.unwrap().unwrap();
}
