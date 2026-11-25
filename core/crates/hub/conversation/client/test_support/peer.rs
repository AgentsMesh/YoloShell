use tokio::{io::DuplexStream, task::JoinHandle, time::Instant};
use yolo_hub_client::Client;
use yolo_hub_identity::HubId;
use yolo_hub_jsonrpc_protocol::{Answer as Response, Call as Request};
use yolo_hub_jsonrpc_transport::{Framed, Incoming};
use yolo_hub_rpc_protocol::{Command, HUB_PROTOCOL_VERSION, Hello, Reply, SUPPORTED_METHODS};

const CHANNEL_CAPACITY: usize = 16384;
const REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(3);

pub fn deadline() -> Instant {
    Instant::now() + REQUEST_TIMEOUT
}

pub async fn connected(replies: Vec<Reply>) -> (Client<DuplexStream>, JoinHandle<Vec<Command>>) {
    let (input, output) = tokio::io::duplex(CHANNEL_CAPACITY);
    let entry = HubId::new();
    let server = tokio::spawn(async move {
        let mut channel = Framed::new(output);
        let Incoming::Message(hello) = channel.receive::<Request>(deadline()).await.unwrap() else {
            panic!()
        };
        channel
            .send(
                &Response::new(
                    &hello,
                    Reply::Hello {
                        hello: Hello {
                            hub_id: entry,
                            version: HUB_PROTOCOL_VERSION,
                            methods: SUPPORTED_METHODS.to_vec(),
                        },
                    },
                )
                .unwrap(),
                deadline(),
            )
            .await
            .unwrap();
        let mut commands = Vec::new();
        for reply in replies {
            let Incoming::Message(request) = channel.receive::<Request>(deadline()).await.unwrap()
            else {
                panic!()
            };
            commands.push(request.fields().command.clone());
            channel
                .send(&Response::new(&request, reply).unwrap(), deadline())
                .await
                .unwrap();
        }
        commands
    });
    (
        Client::open(input, entry, deadline()).await.unwrap(),
        server,
    )
}
