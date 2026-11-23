use super::super::*;
use super::support::*;
use crate::protocol::{RequestEnvelope, method, to_value};
use crate::remote::{RemoteError, RuntimeRole};
use std::sync::Arc;

const FIRST_CONNECTION_GENERATION: u64 = 1;
const SECOND_CONNECTION_GENERATION: u64 = FIRST_CONNECTION_GENERATION + 1;

fn pool(factory: Arc<FakeFactory>) -> BrokerConnectionPool {
    let config = test_config();
    BrokerConnectionPool::new(
        HOST_ID,
        config.revision(),
        config.runtime().clone(),
        factory,
    )
}

#[tokio::test]
async fn concurrent_channels_share_one_authenticated_connection() {
    let connection = FakeConnection::new([]);
    let factory = FakeFactory::new(
        vec![Arc::clone(&connection)],
        vec![ConnectStep::Connection(0)],
    );
    let pool = Arc::new(pool(Arc::clone(&factory)));
    let channels = futures::future::join_all((0..24).map(|_| {
        let pool = Arc::clone(&pool);
        async move { pool.open_runtime(RuntimeRole::Control).await.unwrap() }
    }))
    .await;
    assert_eq!(channels.len(), 24);
    assert_eq!(factory.calls(), 1);
    assert_eq!(connection.opens(), vec![connection.id(); 24]);
    let status = pool.status().await;
    assert_eq!(status.connection_id, Some(connection.id()));
    assert_eq!(status.connection_generation, FIRST_CONNECTION_GENERATION);
}

#[tokio::test]
async fn one_channel_rejection_does_not_replace_or_damage_the_transport() {
    let connection = FakeConnection::new([OpenStep::Rejected, OpenStep::Channel]);
    let factory = FakeFactory::new(
        vec![Arc::clone(&connection)],
        vec![ConnectStep::Connection(0)],
    );
    let pool = pool(Arc::clone(&factory));
    assert!(matches!(
        pool.open_runtime(RuntimeRole::Attachment).await,
        Err(RemoteError::ChannelRejected)
    ));
    pool.open_runtime(RuntimeRole::Attachment).await.unwrap();
    assert_eq!(factory.calls(), 1);
    assert_eq!(pool.status().await.connection_id, Some(connection.id()));
}

#[tokio::test]
async fn transport_loss_reconnects_single_flight_and_new_channels_work() {
    let first = FakeConnection::new([OpenStep::TransportFailure]);
    let second = FakeConnection::new([OpenStep::Channel]);
    let factory = FakeFactory::new(
        vec![Arc::clone(&first), Arc::clone(&second)],
        vec![ConnectStep::Connection(0), ConnectStep::Connection(1)],
    );
    let pool = pool(Arc::clone(&factory));
    pool.open_runtime(RuntimeRole::Control).await.unwrap();
    assert_eq!(factory.calls(), 2);
    let status = pool.status().await;
    assert_eq!(status.connection_id, Some(second.id()));
    assert_eq!(status.connection_generation, SECOND_CONNECTION_GENERATION);
}

#[tokio::test]
async fn runtime_eof_before_caller_request_reconnects_on_a_new_transport() {
    let first = FakeConnection::new([OpenStep::ImmediateEof]);
    let second = FakeConnection::new([OpenStep::Channel]);
    let factory = FakeFactory::new(
        vec![Arc::clone(&first), Arc::clone(&second)],
        vec![ConnectStep::Connection(0), ConnectStep::Connection(1)],
    );
    let pool = pool(Arc::clone(&factory));
    let mut channel = pool.open_runtime(RuntimeRole::Control).await.unwrap();
    let request = RequestEnvelope::new(
        PROXY_REQUEST_ID,
        method::HOST_STATUS,
        to_value(&()).unwrap(),
    );
    let response = channel.exchange(&request, crate::remote::RemoteDeadline::operation().instant()).await.unwrap();
    assert_eq!(response.request_id, request.request_id);
    assert_eq!(factory.calls(), 2);
    assert_eq!(pool.status().await.connection_id, Some(second.id()));
}

#[tokio::test]
async fn eof_after_runtime_probe_is_returned_without_replaying_the_caller_request() {
    let connection = FakeConnection::new([OpenStep::EofAfterProbe]);
    let factory = FakeFactory::new(
        vec![Arc::clone(&connection)],
        vec![ConnectStep::Connection(0)],
    );
    let pool = pool(Arc::clone(&factory));
    let mut channel = pool.open_runtime(RuntimeRole::Control).await.unwrap();
    let request = RequestEnvelope::new(
        PROXY_REQUEST_ID,
        method::HOST_STATUS,
        to_value(&()).unwrap(),
    );
    assert!(channel.exchange(&request, crate::remote::RemoteDeadline::operation().instant()).await.is_err());
    assert_eq!(factory.calls(), 1);
}

#[tokio::test]
async fn reconnect_retries_transport_but_fails_closed_on_authentication() {
    let connection = FakeConnection::new([]);
    let factory = FakeFactory::new(
        vec![connection],
        vec![ConnectStep::TransportFailure, ConnectStep::Connection(0)],
    );
    pool(Arc::clone(&factory))
        .open_runtime(RuntimeRole::Control)
        .await
        .unwrap();
    assert_eq!(factory.calls(), 2);

    let denied = FakeFactory::empty(vec![ConnectStep::AuthenticationFailure]);
    assert!(matches!(
        pool(denied).open_runtime(RuntimeRole::Control).await,
        Err(RemoteError::AuthenticationFailed(_))
    ));
}

#[tokio::test]
async fn mismatched_channel_and_stale_invalidation_are_contained() {
    let first = FakeConnection::new([OpenStep::Mismatch]);
    let second = FakeConnection::new([]);
    let factory = FakeFactory::new(
        vec![Arc::clone(&first), Arc::clone(&second)],
        vec![ConnectStep::Connection(0), ConnectStep::Connection(1)],
    );
    let pool = pool(factory);
    assert!(matches!(
        pool.open_runtime(RuntimeRole::Control).await,
        Err(RemoteError::ChannelMismatch)
    ));
    let stale = pool.observed_connection().await.unwrap();
    pool.invalidate(&stale).await;
    pool.connection().await.unwrap();
    pool.invalidate(&stale).await;
    assert_eq!(pool.status().await.connection_id, Some(second.id()));
}
