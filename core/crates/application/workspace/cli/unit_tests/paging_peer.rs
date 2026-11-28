use crate::constants::RPC_TIMEOUT;
use tokio::{task::JoinHandle, time::Instant};
use yolo_hub_client::LocalClient;
use yolo_hub_identity::HubId;
use yolo_hub_jsonrpc_protocol::{Answer, Call};
use yolo_hub_jsonrpc_transport::{Framed, Incoming};
use yolo_hub_rpc_protocol::{Command, HUB_PROTOCOL_VERSION, Hello, Method, Reply};
use yolo_local_unix::LocalUnixStream;

pub async fn client(
    hub: HubId,
    mut reply: impl FnMut(&Command) -> Reply + Send + 'static,
) -> (LocalClient, JoinHandle<usize>) {
    let (client, server) = LocalUnixStream::pair().unwrap();
    let task = tokio::spawn(async move {
        let mut framed = Framed::new(server);
        let mut count = 0;
        while let Ok(Incoming::Message(call)) =
            framed.receive::<Call>(Instant::now() + RPC_TIMEOUT).await
        {
            let result = match &call.fields().command {
                Command::Hello {} => Reply::Hello {
                    hello: Hello {
                        hub_id: hub,
                        version: HUB_PROTOCOL_VERSION,
                        methods: vec![
                            Method::Hello,
                            Method::ListWorkspaces,
                            Method::List,
                            Method::ListAgentRuns,
                        ],
                    },
                },
                command => {
                    count += 1;
                    reply(command)
                }
            };
            framed
                .send(
                    &Answer::new(&call, result).unwrap(),
                    Instant::now() + RPC_TIMEOUT,
                )
                .await
                .unwrap();
        }
        count
    });
    let client = LocalClient::open(client, hub, Instant::now() + RPC_TIMEOUT)
        .await
        .unwrap();
    (client, task)
}
