use crate::arguments::{Action, Options};
use tokio::{net::UnixListener, task::JoinHandle, time::Instant};
use yolo_hub_identity::HubId;
use yolo_hub_jsonrpc_protocol::{Answer as Response, Call as Request};
use yolo_hub_jsonrpc_transport::{Framed, Incoming};
use yolo_hub_rpc_protocol::{Command, HUB_PROTOCOL_VERSION, Hello, Reply, SUPPORTED_METHODS};

const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(3);

pub fn peer(
    hub: HubId,
    action: Action,
    replies: Vec<Reply>,
) -> (Options, tempfile::TempDir, JoinHandle<Vec<Command>>) {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("conversation.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut framed = Framed::new(stream);
        let deadline = Instant::now() + TIMEOUT;
        let Incoming::Message(request) = framed.receive::<Request>(deadline).await.unwrap() else {
            panic!()
        };
        framed
            .send(
                &Response::new(
                    &request,
                    Reply::Hello {
                        hello: Hello {
                            hub_id: hub,
                            version: HUB_PROTOCOL_VERSION,
                            methods: SUPPORTED_METHODS.to_vec(),
                        },
                    },
                )
                .unwrap(),
                deadline,
            )
            .await
            .unwrap();
        let mut commands = Vec::new();
        for reply in replies {
            let Incoming::Message(request) = framed.receive::<Request>(deadline).await.unwrap()
            else {
                panic!()
            };
            commands.push(request.fields().command.clone());
            framed
                .send(&Response::new(&request, reply).unwrap(), deadline)
                .await
                .unwrap();
        }
        commands
    });
    (
        Options {
            socket: Some(socket),
            hub: Some(hub),
            local_profile: None,
            action,
        },
        directory,
        server,
    )
}
