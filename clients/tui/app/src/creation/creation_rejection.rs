use crate::State;
use std::time::Instant;
use yolo_hub_client::ClientError;
use yolo_hub_client_channels::ControlError;
use yolo_hub_terminal_protocol::{CreateShellRequest, PublicError};

/// An authenticated pre-creation rejection proves the original operation created no resource.
pub(crate) fn proves_uncreated(error: &ControlError) -> bool {
    matches!(
        error,
        ControlError::NotConnected
            | ControlError::StaleGeneration
            | ControlError::Invalidated
            | ControlError::OperationLimit
            | ControlError::Client(ClientError::Remote(PublicError::InvalidDirectory))
    )
}

pub(crate) fn retire(state: &mut State, request: &CreateShellRequest) {
    crate::creation_reservation::rollback(state, request);
    state.creations.cancel_uncreated(request);
    crate::creation_recovery_persistence::flush(state, Instant::now());
}

pub(crate) fn present(state: &mut State, message: String) {
    state.status = message.clone();
    if state.mode == crate::Mode::Directory {
        crate::form_input::fail(state, message);
    }
}
