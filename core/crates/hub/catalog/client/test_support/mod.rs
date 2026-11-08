use tokio::{
    io::{DuplexStream, duplex},
    time::Instant,
};
use yolo_hub_client::Client;
use yolo_hub_identity::HubId;
use yolo_hub_jsonrpc_protocol::{Answer as Response, Call as Request};
use yolo_hub_jsonrpc_transport::{Framed, Incoming};
use yolo_hub_rpc_protocol::{Command, HUB_PROTOCOL_VERSION, Hello, Method, Reply};

pub const PIPE_CAPACITY: usize = 16_384;
pub const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);
pub const FIRST_HUB: u128 = 1;
pub const NEXT_HUB: u128 = 2;

pub fn hub(value: u128) -> HubId {
    HubId::from_uuid(uuid::Uuid::from_u128(value)).unwrap()
}
pub fn deadline() -> Instant {
    Instant::now() + TIMEOUT
}
pub async fn receive(channel: &mut Framed<DuplexStream>) -> Request {
    let Incoming::Message(request) = channel.receive::<Request>(deadline()).await.unwrap() else {
        panic!()
    };
    request
}
pub async fn respond(channel: &mut Framed<DuplexStream>, request: &Request, reply: Reply) {
    channel
        .send(&Response::new(request, reply).unwrap(), deadline())
        .await
        .unwrap();
}
pub async fn connected(methods: Vec<Method>) -> (Client<DuplexStream>, Framed<DuplexStream>) {
    let (left, right) = duplex(PIPE_CAPACITY);
    let server = async {
        let mut channel = Framed::new(right);
        let request = receive(&mut channel).await;
        assert!(matches!(request.fields().command, Command::Hello {}));
        respond(
            &mut channel,
            &request,
            Reply::Hello {
                hello: Hello {
                    hub_id: hub(FIRST_HUB),
                    version: HUB_PROTOCOL_VERSION,
                    methods,
                },
            },
        )
        .await;
        channel
    };
    let (client, server) = tokio::join!(Client::open(left, hub(FIRST_HUB), deadline()), server);
    (client.unwrap(), server)
}

pub async fn empty_sessions(server: &mut Framed<DuplexStream>) {
    let request = receive(server).await;
    assert!(matches!(
        request.fields().command,
        Command::List { after: None, .. }
    ));
    respond(
        server,
        &request,
        Reply::Terminals {
            terminals: vec![],
            next: None,
        },
    )
    .await;
}
