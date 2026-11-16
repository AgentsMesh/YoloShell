use super::super::*;
use super::server_support::Harness;
use super::support::*;
use crate::protocol::{RequestEnvelope, read_frame, write_frame};
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::net::UnixStream;

const CLOSED_STREAM_PROBE_BYTES: usize = 1;
const FENCE_COMPLETION_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(3);
const DELAYED_RUNTIME_REQUEST_ID: &str = "delayed-runtime-during-rotation";

async fn assert_closed(stream: &mut UnixStream) {
    let mut probe = [0_u8; CLOSED_STREAM_PROBE_BYTES];
    let bytes = tokio::time::timeout(FENCE_COMPLETION_TIMEOUT, stream.read(&mut probe))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(bytes, 0);
}

#[tokio::test]
async fn setup_completed_after_rotation_cannot_become_a_runtime_proxy() {
    let connection = FakeConnection::new([OpenStep::BlockedOpen]);
    let factory = FakeFactory::new(
        vec![Arc::clone(&connection)],
        vec![ConnectStep::Connection(0)],
    );
    let harness = Harness::start(factory).await;
    let pending = UnixStream::connect(harness.paths.runtime_socket())
        .await
        .unwrap();
    let request = RequestEnvelope::new(
        DELAYED_RUNTIME_REQUEST_ID,
        crate::protocol::method::HOST_STATUS,
        yolo_capnp_value::Value::Null,
    );
    let pending = super::runtime_client::connect(pending).await;
    let call = super::runtime_client::pending(&pending, request);
    while connection.opens().is_empty() {
        tokio::task::yield_now().await;
    }

    BrokerClient::new(harness.paths.management_socket())
        .shutdown(CONFIG_REVISION.into())
        .await
        .unwrap();
    harness.wait_socket_removed().await;
    connection.probe().release_open();

    assert!(tokio::time::timeout(FENCE_COMPLETION_TIMEOUT, call).await.unwrap().unwrap().is_err());
    drop(pending);
    harness.wait_stopped().await;
}

#[tokio::test]
async fn established_management_connection_cannot_dispatch_after_rotation_cutoff() {
    let factory = FakeFactory::empty(Vec::new());
    let harness = Harness::start(Arc::clone(&factory)).await;
    let mut existing = UnixStream::connect(harness.paths.management_socket())
        .await
        .unwrap();
    let status = BrokerRequestEnvelope::new(BrokerRequest::Status);
    write_frame(&mut existing, &status).await.unwrap();
    let _: BrokerResponse = read_frame(&mut existing).await.unwrap();

    BrokerClient::new(harness.paths.management_socket())
        .shutdown(CONFIG_REVISION.into())
        .await
        .unwrap();
    harness.wait_socket_removed().await;
    let command = BrokerRequestEnvelope::new(BrokerRequest::Command(
        BrokerCommandRequest::standard(ALLOWED_PROGRAM, Vec::new()),
    ));
    write_frame(&mut existing, &command).await.unwrap();

    assert_closed(&mut existing).await;
    assert_eq!(factory.calls(), 0);
    harness.wait_stopped().await;
}
