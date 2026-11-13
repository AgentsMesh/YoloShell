use crate::{ControlError, ControlRequest};
use yolo_hub_rpc_protocol::{Command, Reply};

#[derive(Clone, Copy)]
pub(crate) enum Dispatch {
    NotAttempted,
    Attempted,
}

pub(crate) fn settle(
    request: &ControlRequest,
    result: &Result<Reply, ControlError>,
    dispatch: Dispatch,
) {
    if matches!(request.command, Command::Input { .. })
        && !matches!(result, Ok(Reply::Input { .. }))
    {
        if let Some(validity) = &request.validity {
            // This runs in the serial dispatch owner, before any next request
            // can enter transport. UI completion delivery is not a barrier.
            match dispatch {
                Dispatch::NotAttempted => validity.reject(),
                Dispatch::Attempted => validity.reject_dispatched_input(),
            }
        }
    }
}
