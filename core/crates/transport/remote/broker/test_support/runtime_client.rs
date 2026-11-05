use crate::protocol::{RequestEnvelope, ResponseEnvelope};
use tokio::time::{Duration, Instant};
use yolo_capnp_rpc::{Connection, Error, Method, Protocol};

pub(super) const DEADLINE: Duration = Duration::from_secs(40);
pub(super) async fn assert_business_channel(
    channel: &mut yolo_runtime_rpc::RuntimeConnection,
    connection_id: uuid::Uuid,
) {
    let request = RequestEnvelope::new(
        super::support::PROXY_REQUEST_ID,
        crate::protocol::method::HOST_STATUS,
        yolo_capnp_value::Value::Null,
    );
    let response = channel
        .exchange(
            &request,
            crate::remote::RemoteDeadline::operation().instant(),
        )
        .await
        .unwrap();
    let reply: super::support::ProxyReply = response.decode_payload().unwrap();
    assert_eq!(reply.connection_id, connection_id);
}
pub(super) async fn connect(stream: tokio::net::UnixStream) -> Connection {
    Connection::open(stream, Protocol::Endpoint, Instant::now() + DEADLINE)
        .await
        .unwrap()
}
pub(super) async fn call(
    client: &Connection,
    request: RequestEnvelope,
) -> Result<ResponseEnvelope, Error> {
    let value = yolo_capnp_codec::to_value(&request)?;
    let reply = client
        .request(Method::Call, value, Instant::now() + DEADLINE)
        .await?;
    let response = yolo_capnp_codec::from_value(reply.value)?;
    crate::protocol::validate_response(&request, &response).map_err(|_| Error::Invalid)?;
    Ok(response)
}
pub(super) fn pending(
    client: &Connection,
    request: RequestEnvelope,
) -> tokio::task::JoinHandle<Result<ResponseEnvelope, Error>> {
    let client = client.clone();
    tokio::spawn(async move { call(&client, request).await })
}

pub(super) struct DrivenClock(tokio::task::JoinHandle<()>);
impl DrivenClock {
    pub(super) fn new() -> Self {
        Self(tokio::spawn(async {
            loop {
                tokio::task::yield_now().await;
            }
        }))
    }
}
impl Drop for DrivenClock {
    fn drop(&mut self) {
        self.0.abort();
    }
}
