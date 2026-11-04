use crate::local_unix::LocalUnixStream;
use crate::protocol::{RequestEnvelope, ResponseEnvelope, method, to_value};
use crate::rpc_test_support::Peer;
use tokio::time::{Duration, Instant};
use yolo_runtime_rpc::RuntimeConnection;

const FIXTURE_TIMEOUT: Duration = Duration::from_secs(5);
const FIXTURE_REQUEST_ID: &str = "attachment-fixture";

pub(crate) async fn connected() -> (RuntimeConnection, Peer) {
    let (left, right) = LocalUnixStream::pair().unwrap();
    let mut peer = Peer::new(right);
    let deadline = Instant::now() + FIXTURE_TIMEOUT;
    let mut channel = RuntimeConnection::open(left, deadline).await.unwrap();
    let request = RequestEnvelope::new(
        FIXTURE_REQUEST_ID,
        method::ATTACH_SESSION,
        to_value(&()).unwrap(),
    );
    let server = async {
        let call = peer.request().await.unwrap();
        peer.respond(&ResponseEnvelope::empty(&call)).await.unwrap();
    };
    let (response, ()) = tokio::join!(channel.exchange(&request, deadline), server);
    response.unwrap();
    (channel, peer)
}
