use crate::{ControlError, ControlRequest, constants::CONTROL_KEEPALIVE_INTERVAL};
use tokio::{
    sync::{mpsc, oneshot, watch},
    time::{MissedTickBehavior, interval},
};
use uuid::Uuid;
use yolo_hub_client::EndpointClient;
use yolo_hub_rpc_protocol::Reply;

#[path = "control_dispatch.rs"]
mod dispatch;

pub(crate) enum Command {
    Attach {
        generation: Uuid,
        client: EndpointClient,
    },
    Reset,
    Call {
        request: ControlRequest,
        originals: Vec<ControlRequest>,
    },
}

pub(crate) enum Event {
    Completed(Completion),
    Disconnected { generation: Uuid, message: String },
}

pub(crate) struct Completion {
    pub request: ControlRequest,
    pub originals: Vec<ControlRequest>,
    pub result: Result<Reply, ControlError>,
    pub dispatch: crate::input_completion::Dispatch,
    pub connected: bool,
    pub acknowledged: oneshot::Sender<()>,
}

pub(crate) async fn run(
    mut commands: mpsc::UnboundedReceiver<Command>,
    updates: mpsc::Sender<Event>,
    mut generation: watch::Receiver<Option<Uuid>>,
) {
    let mut client: Option<(Uuid, EndpointClient)> = None;
    let mut pending: Option<oneshot::Receiver<()>> = None;
    let mut idle = interval(CONTROL_KEEPALIVE_INTERVAL);
    idle.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let _ = idle.tick().await;
    loop {
        let command = tokio::select! {
            biased;
            changed = generation.changed() => {
                if changed.is_err() { break; }
                if client.as_ref().is_some_and(|(owner, _)| Some(*owner) != *generation.borrow()) {
                    client = None;
                }
                continue;
            }
            _ = async { pending.as_mut().unwrap().await }, if pending.is_some() => {
                pending = None;
                continue;
            }
            _ = idle.tick(), if client.is_some() => {
                let (owner, current) = client.as_mut().unwrap();
                if let Err(error) = crate::connection_liveness::probe(current).await {
                    let event = Event::Disconnected { generation: *owner, message: error.to_string() };
                    client = None;
                    if updates.send(event).await.is_err() { break; }
                }
                idle.reset();
                continue;
            }
            command = commands.recv(), if pending.is_none() => {
                let Some(command) = command else { break; };
                command
            }
        };
        match command {
            Command::Attach {
                generation,
                client: next,
            } => {
                client = Some((generation, next));
                idle.reset();
            }
            Command::Reset => client = None,
            Command::Call { request, originals } => {
                let (acknowledged, received) = oneshot::channel();
                let completion = dispatch::execute(
                    &mut client,
                    &mut generation,
                    request,
                    originals,
                    acknowledged,
                )
                .await;
                if matches!(
                    completion.dispatch,
                    crate::input_completion::Dispatch::Attempted
                ) {
                    idle.reset();
                }
                if updates.send(Event::Completed(completion)).await.is_err() {
                    break;
                }
                // Keep the actual socket alive while publication is backpressured,
                // but never dispatch the next mutation before its predecessor settles.
                pending = Some(received);
            }
        }
    }
}
