use super::Completion;
use crate::{ControlError, ControlRequest, admission::admit, constants::RPC_TIMEOUT};
use tokio::{
    sync::{oneshot, watch},
    time::Instant,
};
use uuid::Uuid;
use yolo_hub_client::EndpointClient;

pub(super) async fn execute(
    client: &mut Option<(Uuid, EndpointClient)>,
    generation: &mut watch::Receiver<Option<Uuid>>,
    request: ControlRequest,
    originals: Vec<ControlRequest>,
    acknowledged: oneshot::Sender<()>,
) -> Completion {
    let admission = validate(&request, generation);
    let dispatch = if admission.is_ok() && client.is_some() {
        crate::input_completion::Dispatch::Attempted
    } else {
        crate::input_completion::Dispatch::NotAttempted
    };
    let result = match admission {
        Err(error) => Err(error),
        Ok(()) => match client.as_mut() {
            Some((_, current)) => current
                .call(
                    request.destination,
                    request.command.clone(),
                    Instant::now() + RPC_TIMEOUT,
                )
                .await
                .map_err(ControlError::Client),
            None => Err(ControlError::NotConnected),
        },
    };
    let connected = client
        .as_ref()
        .is_some_and(|(_, client)| client.is_usable());
    if !connected {
        *client = None;
    }
    // Settle the serial stream before a queued suffix can enter transport,
    // even when result publication to the UI is delayed.
    crate::input_completion::settle(&request, &result, dispatch);
    Completion {
        request,
        originals,
        result,
        dispatch,
        connected,
        acknowledged,
    }
}

fn validate(
    request: &ControlRequest,
    generation: &watch::Receiver<Option<Uuid>>,
) -> Result<(), ControlError> {
    admit(*generation.borrow(), request.generation)?;
    if request
        .validity
        .as_ref()
        .is_some_and(|validity| !validity.is_current())
    {
        return Err(ControlError::Invalidated);
    }
    Ok(())
}
