use super::super::handlers::{proxy_runtime, proxy_runtime_admitted};
use super::super::*;
use super::{runtime_client, support::*};
use crate::protocol::{RequestEnvelope, method};
use std::sync::Arc;

const FIRST_ID: &str = "first-call";
const SECOND_ID: &str = "second-call";
const ATTACH_ID: &str = "saturated-attach";

fn pool(connection: Arc<FakeConnection>) -> Arc<BrokerConnectionPool> {
    pool_with_connects(vec![connection], vec![ConnectStep::Connection(0)])
}
fn pool_with_connects(
    connections: Vec<Arc<FakeConnection>>,
    connects: Vec<ConnectStep>,
) -> Arc<BrokerConnectionPool> {
    let factory = FakeFactory::new(connections, connects);
    let config = test_config();
    Arc::new(BrokerConnectionPool::new(
        HOST_ID,
        config.revision(),
        config.runtime().clone(),
        factory,
    ))
}
fn request(id: &str) -> RequestEnvelope {
    RequestEnvelope::new(id, method::HOST_STATUS, yolo_capnp_value::Value::Null)
}

#[tokio::test]
async fn pipelined_calls_are_correlated_while_runtime_open_is_pending() {
    let connection = FakeConnection::new([OpenStep::BlockedOpen]);
    let (client, server) = tokio::net::UnixStream::pair().unwrap();
    let proxy = tokio::spawn(proxy_runtime(server.into(), pool(connection.clone())));
    let client = runtime_client::connect(client).await;
    let first = runtime_client::pending(&client, request(FIRST_ID));
    let second = runtime_client::pending(&client, request(SECOND_ID));
    while connection.opens().is_empty() {
        tokio::task::yield_now().await;
    }
    connection.probe().release_open();
    assert_eq!(first.await.unwrap().unwrap().request_id, FIRST_ID);
    assert_eq!(second.await.unwrap().unwrap().request_id, SECOND_ID);
    drop(client);
    proxy.await.unwrap().unwrap();
}

#[tokio::test]
async fn pipelined_calls_are_preserved_while_authenticated_probe_write_is_pending() {
    let connection = FakeConnection::new([OpenStep::GatedHandshakeWrite]);
    let (client, server) = tokio::net::UnixStream::pair().unwrap();
    let proxy = tokio::spawn(proxy_runtime(server.into(), pool(connection.clone())));
    let client = runtime_client::connect(client).await;
    let first = runtime_client::pending(&client, request(FIRST_ID));
    connection.probe().wait_write_pending(1).await;
    let second = runtime_client::pending(&client, request(SECOND_ID));
    connection.probe().release_writes();
    assert_eq!(first.await.unwrap().unwrap().request_id, FIRST_ID);
    assert_eq!(second.await.unwrap().unwrap().request_id, SECOND_ID);
    drop(client);
    proxy.await.unwrap().unwrap();
}

#[tokio::test]
async fn actual_client_disconnect_cancels_pending_open_and_does_not_replay_on_replacement() {
    let retired = FakeConnection::new([OpenStep::BlockedOpen, OpenStep::Rejected]);
    let replacement = FakeConnection::new([OpenStep::Channel]);
    let shared = pool_with_connects(
        vec![retired.clone(), replacement],
        vec![ConnectStep::Connection(0), ConnectStep::Connection(1)],
    );
    let (client, server) = tokio::net::UnixStream::pair().unwrap();
    let proxy = tokio::spawn(proxy_runtime(server.into(), shared.clone()));
    let client = runtime_client::connect(client).await;
    let pending = runtime_client::pending(&client, request(FIRST_ID));
    while retired.opens().is_empty() {
        tokio::task::yield_now().await;
    }
    pending.abort();
    let _ = pending.await;
    drop(client);
    proxy.await.unwrap().unwrap();
    let (client, server) = tokio::net::UnixStream::pair().unwrap();
    let proxy = tokio::spawn(proxy_runtime(server.into(), shared));
    let client = runtime_client::connect(client).await;
    assert!(
        runtime_client::call(&client, request(SECOND_ID))
            .await
            .unwrap()
            .ok
    );
    drop(client);
    proxy.await.unwrap().unwrap();
}

#[tokio::test]
async fn oversized_rpc_payload_is_rejected_without_opening_an_ssh_channel() {
    let connection = FakeConnection::new([OpenStep::BlockedOpen]);
    let (client, server) = tokio::net::UnixStream::pair().unwrap();
    let proxy = tokio::spawn(proxy_runtime(server.into(), pool(connection.clone())));
    let client = runtime_client::connect(client).await;
    let payload = yolo_capnp_codec::Value::Data(vec![0; yolo_capnp_codec::MAX_MESSAGE_BYTES + 1]);
    assert!(
        client
            .request(
                yolo_capnp_rpc::Method::Call,
                payload,
                tokio::time::Instant::now() + runtime_client::DEADLINE
            )
            .await
            .is_err()
    );
    assert!(connection.opens().is_empty());
    drop(client);
    proxy.await.unwrap().unwrap();
}

#[tokio::test]
async fn saturated_runtime_attachments_do_not_block_control_on_the_same_rpc_connection() {
    let attachments = Arc::new(tokio::sync::Semaphore::new(0));
    let connection = FakeConnection::new([OpenStep::Channel]);
    let (client, server) = tokio::net::UnixStream::pair().unwrap();
    let proxy = tokio::spawn(proxy_runtime_admitted(
        server.into(),
        pool(connection.clone()),
        attachments,
    ));
    let client = runtime_client::connect(client).await;
    let attach = RequestEnvelope::new(
        ATTACH_ID,
        method::ATTACH_SESSION,
        yolo_capnp_value::Value::Null,
    );
    let response = runtime_client::call(&client, attach).await.unwrap();
    assert!(!response.ok);
    let error = response.error.unwrap();
    assert_eq!(error.code, crate::protocol::error_code::CONFLICT);
    assert_eq!(
        error.message,
        super::super::constants::RUNTIME_ATTACHMENT_LIMIT_MESSAGE
    );
    assert!(connection.opens().is_empty());
    assert!(
        runtime_client::call(&client, request(FIRST_ID))
            .await
            .unwrap()
            .ok
    );
    drop(client);
    proxy.await.unwrap().unwrap();
}
