use crate::{Mode, State, finalization_state::SAVE_FAILED_LABEL};
use std::time::Instant;
use yolo_hub_client::ClientError;
use yolo_hub_client_channels::{ControlError, ControlRequest};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::PublicError;

/// Only Finalize has the origin's durable, exact-identity receipt contract.
/// These retries never reissue Create, lifecycle commands or terminal input.
pub(crate) fn finished(
    state: &mut State,
    request: &ControlRequest,
    result: Result<Reply, ControlError>,
    now: Instant,
) -> bool {
    let Command::Finalize {
        creation_token,
        creation,
    } = request.command
    else {
        return false;
    };
    let focused_pane = state.tab_layout().focused();
    let Some(entry) = state.creations.finalizing.get_mut(&creation_token) else {
        return false;
    };
    if entry.record.creation != creation
        || request.destination != creation.address.hub_id
        || request
            .generation
            .is_none_or(|generation| generation.is_nil())
        || request.validity.is_none()
    {
        return false;
    }
    if matches!(result, Ok(Reply::Completed {})) {
        // An exact prior attempt's success is sufficient even after reconnect.
        // Removing settlement also retires a queued retry; it changes no view or writer.
        entry.attempt.take();
        entry.acknowledged = true;
        entry.retry_at = now;
        crate::finalization_persistence::acknowledge(state, now);
        return true;
    }
    if entry.acknowledged || !entry.matches_attempt(request) {
        return false;
    }
    entry.retry(now);
    let retry = result.as_ref().is_err_and(retryable);
    if !retry {
        let message = match result {
            Err(error) => error.to_string(),
            Ok(_) => crate::constants::UNEXPECTED_REPLY_LABEL.into(),
        };
        entry.rejection = Some(message.clone());
        let relevant =
            Some(state.tabs.active().id) == entry.tab && Some(focused_pane) == entry.pane;
        if relevant
            && state.focused_terminal() == Some(creation.address)
            && matches!(state.mode, Mode::Inventory | Mode::Terminal)
            && state.tab_manager.is_none()
            && state.hub_manager.is_none()
            && !state.help
        {
            state.status = format!("{SAVE_FAILED_LABEL}: {message}");
        }
    }
    true
}

pub(crate) fn not_sent(state: &mut State, request: &ControlRequest, now: Instant) {
    let Command::Finalize {
        creation_token,
        creation,
    } = request.command
    else {
        return;
    };
    if let Some(entry) = state.creations.finalizing.get_mut(&creation_token) {
        if !entry.acknowledged
            && entry.record.creation == creation
            && request.destination == creation.address.hub_id
            && entry.matches_attempt(request)
        {
            entry.retry(now);
        }
    }
}

fn retryable(error: &ControlError) -> bool {
    match error {
        ControlError::NotConnected
        | ControlError::StaleGeneration
        | ControlError::Invalidated
        | ControlError::OperationLimit
        | ControlError::QueryLimit
        | ControlError::QueryTaskFailed => true,
        ControlError::Client(error) if error.outcome_is_unknown() => true,
        ControlError::Client(
            ClientError::Transport(_)
            | ClientError::Poisoned
            | ClientError::Remote(PublicError::Unreachable | PublicError::Internal),
        ) => true,
        ControlError::Client(_) => false,
    }
}
