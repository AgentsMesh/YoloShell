use super::constants::*;
use tokio::{net::UnixStream, time::Instant};
use yolo_hub_identity::HubId;
use yolo_hub_jsonrpc_protocol::{Answer as Response, Call as Request};
use yolo_hub_jsonrpc_transport::{Framed, Incoming};
use yolo_hub_rpc_protocol::{Command, HUB_PROTOCOL_VERSION, Hello, Method, Reply};

pub type Peer = Framed<UnixStream>;
pub fn deadline() -> Instant {
    Instant::now() + TIMEOUT
}
pub async fn receive(peer: &mut Peer) -> Request {
    let Incoming::Message(request) = peer.receive::<Request>(deadline()).await.unwrap() else {
        panic!()
    };
    request
}
pub async fn reply(peer: &mut Peer, request: &Request, reply: Reply) {
    peer.send(&Response::new(request, reply).unwrap(), deadline())
        .await
        .unwrap();
}
pub async fn hello(peer: &mut Peer, hub: HubId) {
    let request = receive(peer).await;
    assert!(matches!(request.fields().command, Command::Hello {}));
    reply(peer, &request, hello_reply(hub)).await;
}
pub fn hello_reply(hub: HubId) -> Reply {
    Reply::Hello {
        hello: Hello {
            hub_id: hub,
            version: HUB_PROTOCOL_VERSION,
            methods: vec![
                Method::Hello,
                Method::List,
                Method::Input,
                Method::Renew,
                Method::Release,
                Method::Create,
                Method::Finalize,
                Method::QueryCreation,
            ],
        },
    }
}
pub async fn eof(peer: &mut Peer) {
    assert!(matches!(
        peer.receive::<Request>(deadline()).await.unwrap(),
        Incoming::Eof
    ));
}
