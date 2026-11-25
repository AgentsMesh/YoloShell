use super::{server_support::*, support::*, runtime_client};
use crate::protocol::{RequestEnvelope, method};
use std::sync::Arc;
use tokio::net::UnixStream;

#[tokio::test]
async fn pending_remote_write_closes_with_its_client_and_transport_accepts_a_new_channel() {
    let connection = FakeConnection::new([OpenStep::WriteStall, OpenStep::Channel]);
    let factory = FakeFactory::new(vec![connection.clone()], vec![ConnectStep::Connection(0)]);
    let harness = Harness::start_with_limits(Arc::clone(&factory),
        SATURATED_RUNTIME_CLIENT_LIMIT, SATURATED_MANAGEMENT_CLIENT_LIMIT).await;
    let stream = UnixStream::connect(harness.paths.runtime_socket()).await.unwrap();
    let client = runtime_client::connect(stream).await;
    let request = RequestEnvelope::new(PROXY_REQUEST_ID, method::HOST_STATUS, yolo_capnp_value::Value::Null);
    let first = runtime_client::call(&client, request.clone()).await.unwrap();
    assert!(first.ok);
    let first: ProxyReply = first.decode_payload().unwrap();
    let pending = runtime_client::pending(&client, request);
    connection.probe().wait_write_pending(1).await;
    pending.abort(); let _ = pending.await;
    drop(client);
    connection.probe().wait_dropped(1).await;
    let second = proxy_request(harness.paths.runtime_socket(), method::HOST_STATUS).await;
    assert_eq!(first.connection_id, connection.id());
    assert_eq!(second.connection_id, connection.id());
    assert_ne!(first.channel_id, second.channel_id);
    assert_eq!(factory.calls(), 1);
    assert_eq!(connection.opens(), vec![connection.id(); 2]);
    harness.stop().await;
}
