use crate::{ControlError, ControlEvent, Endpoint, constants::*, input_batch::Dispatch};
use tokio::{sync::mpsc, time::Instant};
use yolo_hub_client::{ClientError, EndpointClient};

pub(crate) async fn connect(
    endpoint: &Endpoint,
    commands: &mut mpsc::Receiver<Dispatch>,
    updates: &mpsc::Sender<ControlEvent>,
    queries: &mut crate::queries::Queries,
) -> Option<Result<EndpointClient, ClientError>> {
    let opening = endpoint
        .entry
        .connect(endpoint.hub, Instant::now() + RPC_TIMEOUT);
    tokio::pin!(opening);
    loop {
        tokio::select! {
            biased;
            completed = queries.next(), if !queries.is_empty() => {
                if let Some(completed) = completed {
                    publish_query(updates, completed, false).await;
                }
            }
            command = commands.recv() => {
                let command = command?;
                let originals = match command {
                    Dispatch::Single(request) => vec![request],
                    Dispatch::Inputs { originals, .. } => originals,
                };
                publish_not_connected(updates, originals).await;
            }
            result = &mut opening => return Some(result),
        }
    }
}

async fn publish_not_connected(
    updates: &mpsc::Sender<ControlEvent>,
    originals: Vec<crate::ControlRequest>,
) {
    for request in originals {
        let result = Err(ControlError::NotConnected);
        crate::input_completion::settle(
            &request,
            &result,
            crate::input_completion::Dispatch::NotAttempted,
        );
        let _ = updates
            .send(ControlEvent::Finished {
                request,
                result,
                connected: false,
            })
            .await;
    }
}

pub(crate) async fn publish_query(
    updates: &mpsc::Sender<ControlEvent>,
    completed: crate::operations::Completed,
    connected: bool,
) {
    let _ = updates
        .send(ControlEvent::Finished {
            request: completed.request,
            result: completed.result,
            connected,
        })
        .await;
}
