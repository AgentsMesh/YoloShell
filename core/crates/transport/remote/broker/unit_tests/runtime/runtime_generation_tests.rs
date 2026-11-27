use super::super::handlers::proxy_runtime_admitted;
use super::super::*;
use super::runtime_client::assert_business_channel;
use super::support::*;
use crate::protocol::{RequestEnvelope, method};
use crate::remote::{RemoteError, RuntimeRole};
use std::sync::Arc;
use tokio::sync::Semaphore;

const AVAILABLE_ATTACHMENT_PERMITS: usize = 1;
const CLOSED_SEMAPHORE_PERMITS: usize = 0;
const EXPECTED_PENDING_WRITES: usize = 1;
const EXPECTED_EXHAUSTION_CONNECT_CALLS: usize = 1;
const EXPECTED_RECONNECT_CALLS: usize = 2;
const EXPECTED_STALE_CHANNEL_DROPS: usize = 1;
const FIRST_CONNECTION_INDEX: usize = 0;
const NO_AVAILABLE_ATTACHMENT_PERMITS: usize = 0;
const SINGLE_SIGNAL_PERMIT: usize = 1;

fn pool(factory: Arc<FakeFactory>) -> Arc<BrokerConnectionPool> {
    let config = test_config();
    Arc::new(BrokerConnectionPool::new(
        HOST_ID,
        config.revision(),
        config.runtime().clone(),
        factory,
    ))
}

fn reconnecting_factory(connection: Arc<FakeConnection>) -> Arc<FakeFactory> {
    FakeFactory::new(
        vec![connection],
        vec![
            ConnectStep::Connection(FIRST_CONNECTION_INDEX),
            ConnectStep::Connection(FIRST_CONNECTION_INDEX),
        ],
    )
}

fn assert_timeout(error: &RemoteError) {
    assert!(matches!(
        error,
        RemoteError::Transport(message) if message == crate::remote::OPERATION_TIMEOUT_MESSAGE
    ));
}

#[tokio::test(start_paused = true)]
async fn timeout_retires_generation_and_releases_permit_while_state_lock_is_held() {
    let _clock = super::runtime_client::DrivenClock::new();
    let connection = FakeConnection::new([OpenStep::GatedHandshakeWrite, OpenStep::Channel]);
    let shared = pool(reconnecting_factory(connection.clone()));
    let attachments = Arc::new(Semaphore::new(AVAILABLE_ATTACHMENT_PERMITS));
    let (client, server) = tokio::net::UnixStream::pair().unwrap();
    let proxy_pool = Arc::clone(&shared);
    let proxy_attachments = Arc::clone(&attachments);
    let proxy = tokio::spawn(async move {
        proxy_runtime_admitted(server.into(), proxy_pool, proxy_attachments).await
    });
    let request = RequestEnvelope::new(
        PROXY_REQUEST_ID,
        method::ATTACH_SESSION,
        yolo_capnp_value::Value::Null,
    );
    let client = super::runtime_client::connect(client).await;
    let pending = super::runtime_client::pending(&client, request);
    connection
        .probe()
        .wait_write_pending(EXPECTED_PENDING_WRITES)
        .await;
    let observed = shared.observed_connection().await.unwrap();
    assert_eq!(
        attachments.available_permits(),
        NO_AVAILABLE_ATTACHMENT_PERMITS
    );

    let entered = Arc::new(Semaphore::new(CLOSED_SEMAPHORE_PERMITS));
    let release = Arc::new(Semaphore::new(CLOSED_SEMAPHORE_PERMITS));
    let blocker_pool = Arc::clone(&shared);
    let blocker_entered = Arc::clone(&entered);
    let blocker_release = Arc::clone(&release);
    let blocker = tokio::spawn(async move {
        blocker_pool
            .block_state_updates(blocker_entered, blocker_release)
            .await;
    });
    entered.acquire().await.unwrap().forget();
    tokio::time::advance(crate::remote::SSH_OPERATION_TIMEOUT).await;

    let error = pending.await.unwrap().unwrap_err();
    assert!(matches!(error, yolo_capnp_rpc::Error::OutcomeUnknown));
    assert!(shared.is_retired(&observed));
    assert_eq!(
        attachments.available_permits(),
        AVAILABLE_ATTACHMENT_PERMITS
    );
    release.add_permits(SINGLE_SIGNAL_PERMIT);
    blocker.await.unwrap();
    assert!(shared.status().await.connection_id.is_none());
    drop(client);
    proxy.await.unwrap().unwrap();
}

#[tokio::test(start_paused = true)]
async fn stale_timeout_cannot_retire_a_new_same_id_generation_or_channel() {
    let _clock = super::runtime_client::DrivenClock::new();
    let connection = FakeConnection::new([OpenStep::GatedHandshakeWrite, OpenStep::Channel]);
    let shared = pool(reconnecting_factory(connection.clone()));
    let first_pool = Arc::clone(&shared);
    let first = tokio::spawn(async move { first_pool.open_runtime(RuntimeRole::Control).await });
    connection
        .probe()
        .wait_write_pending(EXPECTED_PENDING_WRITES)
        .await;
    let stale = shared.observed_connection().await.unwrap();
    shared.invalidate(&stale).await;

    let mut fresh = shared.open_runtime(RuntimeRole::Control).await.unwrap();
    let fresh_status = shared.status().await;
    assert_eq!(fresh_status.connection_id, Some(connection.id()));
    assert!(fresh_status.connection_generation > stale.generation());
    tokio::time::advance(crate::remote::SSH_OPERATION_TIMEOUT).await;
    assert_timeout(&first.await.unwrap().err().unwrap());

    assert_business_channel(&mut fresh, connection.id()).await;
    assert_eq!(
        shared.status().await.connection_generation,
        fresh_status.connection_generation
    );
}

#[tokio::test(start_paused = true)]
async fn successful_preflight_on_a_retired_generation_is_never_published() {
    let _clock = super::runtime_client::DrivenClock::new();
    let connection = FakeConnection::new([OpenStep::GatedHandshakeWrite, OpenStep::Channel]);
    let factory = reconnecting_factory(connection.clone());
    let shared = pool(factory.clone());
    let task_pool = Arc::clone(&shared);
    let task = tokio::spawn(async move { task_pool.open_runtime(RuntimeRole::Control).await });
    connection
        .probe()
        .wait_write_pending(EXPECTED_PENDING_WRITES)
        .await;
    let stale = shared.observed_connection().await.unwrap();
    shared.invalidate(&stale).await;
    connection.probe().release_writes();
    connection
        .probe()
        .wait_dropped(EXPECTED_STALE_CHANNEL_DROPS)
        .await;
    tokio::time::advance(super::super::constants::INITIAL_RECONNECT_DELAY).await;

    let mut fresh = task.await.unwrap().unwrap();
    assert_eq!(factory.calls(), EXPECTED_RECONNECT_CALLS);
    assert!(shared.status().await.connection_generation > stale.generation());
    assert_business_channel(&mut fresh, connection.id()).await;
}

#[tokio::test]
async fn exhausted_generation_fails_closed_without_publishing_connection() {
    let connection = FakeConnection::new([]);
    let factory = FakeFactory::new(
        vec![connection],
        vec![ConnectStep::Connection(FIRST_CONNECTION_INDEX)],
    );
    let shared = pool(factory.clone());
    shared.set_generation_for_test(u64::MAX).await;

    let error = match shared.observed_connection().await {
        Ok(_) => panic!("exhausted generation must not publish a connection"),
        Err(error) => error,
    };
    assert!(matches!(
        error,
        RemoteError::Transport(message)
            if message == super::super::constants::CONNECTION_GENERATION_EXHAUSTED_MESSAGE
    ));
    let status = shared.status().await;
    assert_eq!(status.connection_generation, u64::MAX);
    assert!(status.connection_id.is_none());
    assert_eq!(factory.calls(), EXPECTED_EXHAUSTION_CONNECT_CALLS);
}
