use super::recovery_constants::*;
use std::sync::{Arc, Mutex};
use tokio::net::{UnixListener, UnixStream};
use yolo_hub_build_protocol::ServiceBuild;
use yolo_hub_jsonrpc_protocol::{Answer as Response, Call as Request};
use yolo_hub_jsonrpc_transport::{Framed, Incoming};
use yolo_hub_management_protocol::ConfigurationSnapshot;
use yolo_hub_rpc_protocol::{HUB_PROTOCOL_VERSION, Hello, Method, Reply};
use yolo_hub_state::HubState;
#[path = "recovery_reply.rs"]
mod reply;

pub struct State {
    pub snapshot: ConfigurationSnapshot,
    pub build: ServiceBuild,
    pub methods: Vec<Method>,
    pub lose_reply: bool,
    pub before_snapshot: Option<Box<dyn FnOnce() + Send>>,
    pub disconnect_after: Option<Method>,
    pub connections: usize,
}
pub struct Peer {
    stop: Option<tokio::sync::oneshot::Sender<()>>,
    task: Option<tokio::task::JoinHandle<()>>,
}
impl Peer {
    pub async fn start(hub: Arc<HubState>, state: Arc<Mutex<State>>) -> Self {
        let (listener, publication) = yolo_paths::socket_activation::publish(
            hub.socket_directory().unwrap(),
            &hub.socket_path(),
            TIMEOUT,
            LISTENER_BUSY,
        )
        .await
        .unwrap();
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            tokio::select! {
                _ = stopped => {},
                _ = serve(listener, &hub, state) => {},
            }
            publication.withdraw().unwrap();
        });
        Self {
            stop: Some(stop),
            task: Some(task),
        }
    }
    pub async fn finish(&mut self) {
        let _ = self.stop.take().unwrap().send(());
        self.task.take().unwrap().await.unwrap();
    }
}
impl Drop for Peer {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}
async fn serve(listener: UnixListener, hub: &HubState, state: Arc<Mutex<State>>) {
    loop {
        let (stream, _) = listener.accept().await.unwrap();
        state.lock().unwrap().connections += 1;
        let mut channel = Framed::new(stream);
        let hello = receive(&mut channel).await.unwrap();
        send(
            &mut channel,
            &hello,
            Reply::Hello {
                hello: Hello {
                    hub_id: hub.hub_id(),
                    version: HUB_PROTOCOL_VERSION,
                    methods: vec![
                        Method::Hello,
                        Method::GetBuild,
                        Method::GetConfiguration,
                        Method::Configure,
                    ],
                },
            },
        )
        .await;
        while let Some(request) = receive(&mut channel).await {
            let (response, disconnect) = {
                let mut state = state.lock().unwrap();
                let disconnect = state.disconnect_after == Some(request.fields().command.method());
                if disconnect {
                    state.disconnect_after = None;
                }
                (reply::respond(&mut state, &request), disconnect)
            };
            let Some(reply) = response else {
                break;
            };
            send(&mut channel, &request, reply).await;
            if disconnect {
                break;
            }
        }
        // None deliberately closes the actual connection after the mutation was applied.
    }
}
async fn receive(channel: &mut Framed<UnixStream>) -> Option<Request> {
    match channel.receive(deadline()).await {
        Ok(Incoming::Message(request)) => Some(request),
        Ok(Incoming::Eof) => None,
        Err(error) => panic!("{error}"),
    }
}
async fn send(channel: &mut Framed<UnixStream>, request: &Request, reply: Reply) {
    channel
        .send(&Response::new(request, reply).unwrap(), deadline())
        .await
        .unwrap();
}
