use crate::{ControlError, ControlEvent, ControlRequest, Endpoint, admission::admit, constants::*};
use tokio::{
    sync::{mpsc, watch},
    time::{Instant, sleep},
};
use uuid::Uuid;
use yolo_hub_client::EndpointClient;
use yolo_hub_rpc_protocol::Command;

/// Lease heartbeats have their own transport, so blocked ordered input cannot
/// consume the writer's renewal window. This owner never retries a mutation.
pub(crate) async fn run(
    endpoint: Endpoint,
    mut commands: mpsc::Receiver<ControlRequest>,
    updates: mpsc::Sender<ControlEvent>,
    mut generation: watch::Receiver<Option<Uuid>>,
) {
    let mut client: Option<EndpointClient> = None;
    let mut connected_generation = None;
    let mut idle = Box::pin(sleep(CONTROL_KEEPALIVE_INTERVAL));
    loop {
        let request = tokio::select! {
            biased;
            result = generation.changed() => {
                client = None;
                connected_generation = None;
                if result.is_err() { break; }
                continue;
            }
            _ = &mut idle, if client.is_some() => {
                // No active renewals need this connection. Reconnect before a
                // future mutation instead of discovering idle expiry after send.
                client = None;
                connected_generation = None;
                continue;
            }
            request = commands.recv() => match request { Some(request) => request, None => break },
        };
        let deadline = Instant::now() + RPC_TIMEOUT;
        let result = async {
            validate(&request, *generation.borrow())?;
            if connected_generation != request.generation {
                client = None;
            }
            if client.is_none() {
                client = Some(endpoint.entry.connect(endpoint.hub, deadline).await?);
                connected_generation = request.generation;
            }
            validate(&request, *generation.borrow())?;
            client
                .as_mut()
                .unwrap()
                .call(request.destination, request.command.clone(), deadline)
                .await
                .map_err(ControlError::Client)
        }
        .await;
        // Invalid local requests must not keep an otherwise idle socket cached.
        if !matches!(
            result,
            Err(ControlError::Invalidated
                | ControlError::StaleGeneration
                | ControlError::NotConnected)
        ) {
            idle.as_mut()
                .reset(Instant::now() + CONTROL_KEEPALIVE_INTERVAL);
        }
        if client.as_ref().is_some_and(|client| !client.is_usable()) {
            client = None;
            connected_generation = None;
        }
        let connected = generation.borrow().is_some();
        if updates
            .send(ControlEvent::Finished {
                request,
                result,
                connected,
            })
            .await
            .is_err()
        {
            break;
        }
    }
}

fn validate(request: &ControlRequest, current: Option<Uuid>) -> Result<(), ControlError> {
    admit(current, request.generation)?;
    if !matches!(request.command, Command::Renew { .. })
        || request
            .validity
            .as_ref()
            .is_some_and(|validity| !validity.is_current())
    {
        return Err(ControlError::Invalidated);
    }
    Ok(())
}
