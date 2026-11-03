use super::constants::NETWORK_TEST_TIMEOUT;
use tokio::{net::UnixStream, time::Instant};
use yolo_hub_identity::HubId;
use yolo_hub_jsonrpc_protocol::{Answer as Response, Call as Request};
use yolo_hub_jsonrpc_transport::{Framed, Incoming};
use yolo_hub_rpc_protocol::{Command, HUB_PROTOCOL_VERSION, Hello, Method, Reply};

pub type Peer = Framed<UnixStream>;
pub fn deadline() -> Instant {
    Instant::now() + NETWORK_TEST_TIMEOUT
}
pub async fn receive(peer: &mut Peer) -> Request {
    let Incoming::Message(request) = peer.receive::<Request>(deadline()).await.unwrap() else {
        panic!()
    };
    request
}
pub async fn respond(peer: &mut Peer, request: &Request, reply: Reply) {
    peer.send(&Response::new(request, reply).unwrap(), deadline())
        .await
        .unwrap();
}
pub async fn hello(peer: &mut Peer, hub: HubId) {
    let request = receive(peer).await;
    assert!(matches!(request.fields().command, Command::Hello {}));
    respond(
        peer,
        &request,
        Reply::Hello {
            hello: Hello {
                hub_id: hub,
                version: HUB_PROTOCOL_VERSION,
                methods: vec![Method::Hello, Method::List],
            },
        },
    )
    .await;
}
