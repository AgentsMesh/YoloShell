use std::path::PathBuf;
use tokio::{net::UnixListener, task::JoinHandle, time::Instant};
use yolo_hub_identity::HubId;
use yolo_hub_jsonrpc_protocol::{Answer as Response, Call as Request};
use yolo_hub_jsonrpc_transport::{Framed, Incoming};
use yolo_hub_rpc_protocol::{Command, HUB_PROTOCOL_VERSION, Hello, Method, Reply};

const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(3);

pub const METHODS: &[Method] = &[
    Method::Create,
    Method::Finalize,
    Method::TerminalRun,
    Method::AgentStart,
    Method::Get,
    Method::List,
    Method::ListAgents,
    Method::WaitAgents,
    Method::ListHubs,
    Method::GetAgent,
    Method::GetAgentRun,
    Method::ListAgentRuns,
    Method::Lease,
    Method::Release,
    Method::Input,
    Method::AgentControl,
    Method::ResolveConversation,
    Method::ReadConversation,
    Method::WatchConversation,
];

pub fn peer<F>(hub: HubId, mut respond: F) -> (PathBuf, tempfile::TempDir, JoinHandle<Vec<Command>>)
where
    F: FnMut(&Command) -> Reply + Send + 'static,
{
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("agent.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut framed = Framed::new(stream);
        let deadline = Instant::now() + TIMEOUT;
        let Incoming::Message(request) = framed.receive::<Request>(deadline).await.unwrap() else {
            panic!("expected hello request")
        };
        framed
            .send(
                &Response::new(
                    &request,
                    Reply::Hello {
                        hello: Hello {
                            hub_id: hub,
                            version: HUB_PROTOCOL_VERSION,
                            methods: METHODS.to_vec(),
                        },
                    },
                )
                .unwrap(),
                deadline,
            )
            .await
            .unwrap();
        let mut commands = Vec::new();
        loop {
            let deadline = Instant::now() + TIMEOUT;
            let Ok(Incoming::Message(request)) = framed.receive::<Request>(deadline).await else {
                break;
            };
            let command = request.fields().command.clone();
            commands.push(command.clone());
            let reply = respond(&command);
            if framed
                .send(&Response::new(&request, reply).unwrap(), deadline)
                .await
                .is_err()
            {
                break;
            }
        }
        commands
    });
    (socket, directory, server)
}
