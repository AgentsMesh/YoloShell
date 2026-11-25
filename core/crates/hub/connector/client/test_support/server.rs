use super::fixtures::{constants::*, *};
use crate::Client;
use tokio::io::{AsyncRead, AsyncWrite, DuplexStream, duplex};
use yolo_hub_connector_jsonrpc::{Answer, Call};
use yolo_hub_connector_protocol::{Command, Reply};
use yolo_hub_jsonrpc_transport::{Framed, Incoming};

pub async fn receive<S: AsyncRead + Unpin>(server: &mut Framed<S>) -> Call {
    let Incoming::Message(request) = server.receive::<Call>(deadline()).await.unwrap() else {
        panic!()
    };
    request
}
pub async fn respond<S: AsyncWrite + Unpin>(server: &mut Framed<S>, request: &Call, reply: Reply) {
    server
        .send(&Answer::new(request, reply).unwrap(), deadline())
        .await
        .unwrap();
}
pub async fn handshake<S: AsyncRead + AsyncWrite + Unpin>(server: &mut Framed<S>) {
    let request = receive(server).await;
    assert_eq!(request.hub, hub());
    assert!(matches!(request.command, Command::Hello {}));
    respond(server, &request, Reply::Hello { hub: hub() }).await;
}
pub async fn connected() -> (Client<DuplexStream>, Framed<DuplexStream>) {
    let (left, right) = duplex(PIPE_CAPACITY);
    let peer = async {
        let mut server = Framed::new(right);
        handshake(&mut server).await;
        server
    };
    let (client, server) = tokio::join!(Client::open(left, hub(), deadline()), peer);
    (client.unwrap(), server)
}
pub async fn eof<S: AsyncRead + Unpin>(server: &mut Framed<S>) {
    assert_eq!(
        server.receive::<Call>(deadline()).await.unwrap(),
        Incoming::Eof
    );
}
