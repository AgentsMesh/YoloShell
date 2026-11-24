use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tokio::time::{Duration, Instant};
use yolo_hub_peer_protocol::{ExecutionReply, PeerReply, PeerRequest, Purpose, Signed};
use yolo_hub_peer_protocol_test_support::Fixture;
use yolo_hub_peer_rpc::{Client, Handler, Response, RpcError, RpcFuture};

const TIMEOUT: Duration = Duration::from_secs(3);
const PIPE_BYTES: usize = 65_536;
struct Origin {
    fixture: Fixture,
    calls: Arc<AtomicUsize>,
}
impl Handler for Origin {
    fn request(&self, request: PeerRequest) -> RpcFuture<'_, Response> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Box::pin(async move {
            let PeerRequest::Execute { call } = request else {
                return Err(RpcError::Invalid);
            };
            call.verify(Purpose::Call, self.fixture.entry.identity)
                .map_err(|_| RpcError::Invalid)?;
            let response = yolo_hub_rpc_protocol::Response::new(
                &call.payload.request,
                self.fixture.origin.identity.hub(),
                yolo_hub_rpc_protocol::Reply::Terminal {
                    terminal: self.fixture.export.catalogue.payload.terminals[0].clone(),
                },
            );
            let response = Signed::new(
                ExecutionReply {
                    call_fingerprint: call.fingerprint().map_err(|_| RpcError::Invalid)?,
                    response,
                },
                Purpose::Response,
                &self.fixture.origin,
            )
            .map_err(|_| RpcError::Invalid)?;
            Ok(Response {
                reply: PeerReply::Executed { response },
                updates: None,
            })
        })
    }
}

#[tokio::test]
async fn native_peer_rpc_preserves_signed_call_and_response_correlation() {
    let fixture = Fixture::new();
    let call = fixture.call();
    let calls = Arc::new(AtomicUsize::new(0));
    let (client, server) = tokio::io::duplex(PIPE_BYTES);
    let server = tokio::spawn(yolo_hub_peer_rpc::serve(
        server,
        Arc::new(Origin {
            fixture,
            calls: calls.clone(),
        }),
    ));
    let client = Client::connect(client, Instant::now() + TIMEOUT)
        .await
        .unwrap();
    let received = client
        .request(
            PeerRequest::Execute { call: call.clone() },
            Instant::now() + TIMEOUT,
        )
        .await
        .unwrap();
    let PeerReply::Executed { response } = received.reply else {
        panic!("expected a signed execution reply")
    };
    response.validate_for(&call).unwrap();
    assert!(received.updates.is_none());
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    drop(client);
    tokio::time::timeout(TIMEOUT, server)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn discover_cannot_smuggle_an_execute_payload_into_the_handler() {
    let fixture = Fixture::new();
    let call = fixture.call();
    let calls = Arc::new(AtomicUsize::new(0));
    let (client, server) = tokio::io::duplex(PIPE_BYTES);
    let server = tokio::spawn(yolo_hub_peer_rpc::serve(
        server,
        Arc::new(Origin {
            fixture,
            calls: calls.clone(),
        }),
    ));
    let client = yolo_capnp_rpc::Connection::open(
        client,
        yolo_capnp_rpc::Protocol::Peer,
        Instant::now() + TIMEOUT,
    )
    .await
    .unwrap();
    let value = yolo_capnp_codec::to_value(&PeerRequest::Execute { call }).unwrap();
    assert!(
        client
            .request(
                yolo_capnp_rpc::Method::Discover,
                value,
                Instant::now() + TIMEOUT
            )
            .await
            .is_err()
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    drop(client);
    let _ = tokio::time::timeout(TIMEOUT, server).await.unwrap();
}
