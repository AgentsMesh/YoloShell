use super::super::BrokerConnectionPool;
use super::super::handlers::proxy_runtime;
use super::support::*;
use crate::local_unix::UnexpectedAncillaryData;
use crate::local_unix::test_support::{assert_reader_eof, pipe, send_right};
use crate::protocol::{RequestEnvelope, method};
use std::future::Future;
use std::os::fd::AsRawFd;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const ANCILLARY_MARKER: u8 = b'R';
const TRANSPORT_SETTLEMENT_TIMEOUT: Duration = Duration::from_secs(2);
const TRANSPORT_SETTLEMENT_TIMEOUT_MESSAGE: &str = "runtime transport test did not settle";
const FIRST_CONNECTION_INDEX: usize = 0;
const NO_COMPLETED_OPENS: usize = 0;
const NO_CONNECTION_ATTEMPTS: usize = 0;
const SINGLE_CONNECTION_ATTEMPT: usize = 1;
const SINGLE_OPEN_ATTEMPT: usize = 1;
const RUNTIME_CHANNEL_TEST_BYTES: usize = 16;

fn pool(connection: Arc<FakeConnection>) -> (Arc<BrokerConnectionPool>, Arc<FakeFactory>) {
    let factory = FakeFactory::new(
        vec![connection],
        vec![ConnectStep::Connection(FIRST_CONNECTION_INDEX)],
    );
    let config = test_config();
    let pool = Arc::new(BrokerConnectionPool::new(
        HOST_ID,
        config.revision(),
        config.runtime().clone(),
        factory.clone(),
    ));
    (pool, factory)
}

fn assert_ancillary_failure(error: &anyhow::Error) {
    assert!(
        error
            .to_string()
            .contains(&UnexpectedAncillaryData.to_string()),
        "{error:#}"
    );
}

fn assert_rpc_stream_rejection(error: &anyhow::Error) {
    // Cap'n Proto polls both halves: its read-side ancillary error or the
    // subsequent poisoned write can complete first. Neither is a remote effect.
    assert!(
        matches!(
            error.downcast_ref::<yolo_capnp_rpc::Error>(),
            Some(yolo_capnp_rpc::Error::Remote(_))
        ),
        "{error:#}"
    );
}

async fn assert_socket_eof(stream: &mut tokio::net::UnixStream) {
    let mut byte = u8::default();
    assert_eq!(
        settle(stream.read(std::slice::from_mut(&mut byte)))
            .await
            .unwrap(),
        usize::default()
    );
}

async fn settle<F: Future>(future: F) -> F::Output {
    tokio::time::timeout(TRANSPORT_SETTLEMENT_TIMEOUT, future)
        .await
        .expect(TRANSPORT_SETTLEMENT_TIMEOUT_MESSAGE)
}

#[tokio::test]
async fn runtime_rejects_a_right_before_the_first_frame_without_opening_ssh() {
    let connection = FakeConnection::new([]);
    let (pool, factory) = pool(Arc::clone(&connection));
    let (mut client, server) = tokio::net::UnixStream::pair().unwrap();
    let transferred = pipe();
    send_right(
        client.as_raw_fd(),
        transferred.writer.as_raw_fd(),
        ANCILLARY_MARKER,
    );
    drop(transferred.writer);

    let error = settle(proxy_runtime(server.into(), pool))
        .await
        .unwrap_err();

    assert_rpc_stream_rejection(&error);
    assert_socket_eof(&mut client).await;
    assert_reader_eof(&transferred.reader);
    assert!(connection.opens().is_empty());
    assert_eq!(connection.completed_opens(), NO_COMPLETED_OPENS);
    assert_eq!(factory.calls(), NO_CONNECTION_ATTEMPTS);
}

#[tokio::test]
async fn pending_runtime_open_rejects_a_right_without_completing_or_reusing_a_channel() {
    let connection = FakeConnection::new([OpenStep::BlockedOpen]);
    let (pool, factory) = pool(Arc::clone(&connection));
    let (client, server) = tokio::net::UnixStream::pair().unwrap();
    let proxy = tokio::spawn(proxy_runtime(server.into(), pool));
    let descriptor = client.as_raw_fd();
    let client = super::runtime_client::connect(client).await;
    let request = RequestEnvelope::new(
        PROXY_REQUEST_ID,
        method::HOST_STATUS,
        yolo_capnp_value::Value::Null,
    );
    let pending = super::runtime_client::pending(&client, request);
    settle(async {
        while connection.opens().is_empty() {
            tokio::task::yield_now().await;
        }
    })
    .await;
    let transferred = pipe();
    send_right(descriptor, transferred.writer.as_raw_fd(), ANCILLARY_MARKER);
    drop(transferred.writer);

    let error = settle(proxy).await.unwrap().unwrap_err();

    assert_rpc_stream_rejection(&error);
    assert!(pending.await.unwrap().is_err());
    drop(client);
    assert_reader_eof(&transferred.reader);
    assert_eq!(
        connection.opens(),
        vec![connection.id(); SINGLE_OPEN_ATTEMPT]
    );
    assert_eq!(connection.completed_opens(), NO_COMPLETED_OPENS);
    assert_eq!(factory.calls(), SINGLE_CONNECTION_ATTEMPT);
}

#[tokio::test]
async fn established_runtime_rejects_a_right_without_forwarding_its_marker() {
    let (mut client, proxy_local) = tokio::net::UnixStream::pair().unwrap();
    let (proxy_remote, mut remote) = tokio::io::duplex(RUNTIME_CHANNEL_TEST_BYTES);
    remote.shutdown().await.unwrap();
    let transferred = pipe();
    send_right(
        client.as_raw_fd(),
        transferred.writer.as_raw_fd(),
        ANCILLARY_MARKER,
    );
    drop(transferred.writer);
    client.shutdown().await.unwrap();

    let error = settle(super::super::runtime_forward::forward(
        proxy_local.into(),
        Box::new(proxy_remote),
        Vec::new(),
    ))
    .await
    .unwrap_err();

    assert_ancillary_failure(&error);
    let mut forwarded = Vec::new();
    settle(remote.read_to_end(&mut forwarded)).await.unwrap();
    assert!(forwarded.is_empty());
    assert_reader_eof(&transferred.reader);
}
