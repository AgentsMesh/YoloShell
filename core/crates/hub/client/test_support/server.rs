use super::fixtures::{constants::*, *};
use crate::Client;
use tokio::io::{AsyncRead, AsyncWrite, DuplexStream, duplex};
use yolo_hub_jsonrpc_protocol::{Answer as Response, Call as Request};
use yolo_hub_jsonrpc_transport::{Framed, Incoming};
use yolo_hub_rpc_protocol::{
    Command, HUB_PROTOCOL_VERSION, Hello, Method, Reply, SUPPORTED_METHODS,
};

pub async fn receive<S: AsyncRead + Unpin>(channel: &mut Framed<S>) -> Request {
    let Incoming::Message(request) = channel.receive::<Request>(deadline()).await.unwrap() else {
        panic!()
    };
    request
}

pub async fn respond<S: AsyncWrite + Unpin>(
    channel: &mut Framed<S>,
    request: &Request,
    reply: Reply,
) {
    if let Reply::Attached { attachment } = reply {
        yolo_hub_terminal_wire::send_attachment(channel, request, &attachment, deadline())
            .await
            .unwrap();
        return;
    }
    channel
        .send(&Response::new(request, reply).unwrap(), deadline())
        .await
        .unwrap();
}

pub async fn hello<S: AsyncRead + AsyncWrite + Unpin>(
    channel: &mut Framed<S>,
    methods: Vec<Method>,
) {
    let request = receive(channel).await;
    assert!(matches!(request.fields().command, Command::Hello {}));
    respond(
        channel,
        &request,
        Reply::Hello {
            hello: Hello {
                hub_id: hub(),
                version: HUB_PROTOCOL_VERSION,
                methods,
            },
        },
    )
    .await;
}

pub async fn connected() -> (Client<DuplexStream>, Framed<DuplexStream>) {
    connected_with(SUPPORTED_METHODS.to_vec()).await
}

pub async fn connected_with(methods: Vec<Method>) -> (Client<DuplexStream>, Framed<DuplexStream>) {
    let (left, right) = duplex(PIPE_CAPACITY);
    let server = async {
        let mut channel = Framed::new(right);
        hello(&mut channel, methods).await;
        channel
    };
    let (client, server) = tokio::join!(Client::open(left, hub(), deadline()), server);
    (client.unwrap(), server)
}
