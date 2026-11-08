use crate::State;
use yolo_hub_client_channels::{ControlError, ControlRequest};
use yolo_hub_rpc_protocol::{Command, Reply};

pub(crate) fn finished(
    state: &mut State,
    request: &ControlRequest,
    result: Option<&Result<Reply, ControlError>>,
) -> Option<bool> {
    let Command::Input { request: input } = &request.command else {
        return None;
    };
    let id = input.fields().request_id;
    let Some(expected) = state.inputs.pending.get(&id) else {
        return None;
    };
    if !matches_request(expected, request) {
        return Some(false);
    }
    state.inputs.settle_pending(id);
    let address = input.fields().grant.fields().target.address;
    if matches!(result, Some(Ok(Reply::Input { .. }))) {
        // A successful receipt proves the bytes were accepted. If this was the
        // last pending request for the address and nothing failed, the earlier
        // authority change left no uncertain prefix: the operator no longer
        // needs a manual resume.
        if !state.inputs.failures.contains(&address) && !state.inputs.pending_for(address) {
            state.inputs.interrupted.remove(&address);
            state.inputs.resume_requested.remove(&address);
            if state.focused_terminal() == Some(address)
                && state.status == crate::constants::INPUT_WAITING_LABEL
            {
                state.status = crate::constants::CONNECTED_LABEL.into();
            }
        }
    } else {
        state.inputs.failures.insert(address);
    }
    if !matches!(result, Some(Ok(Reply::Input { .. }))) {
        if let Some(validity) = &request.validity {
            if matches!(result, Some(Err(ControlError::Client(_))) | Some(Ok(_))) {
                validity.reject_dispatched_input();
            } else {
                validity.reject();
            }
        }
    }
    let grant = input.fields().grant;
    if request.generation != state.control_generation
        || !state.inputs.matches_stream(request.generation, grant)
    {
        return Some(true);
    }
    // Navigation proves queued cancellation only for known-unsent outcomes.
    // An in-flight unknown result still belongs to this exact writer stream.
    if !state.inputs.matches_scope(request)
        && matches!(result, None | Some(Err(ControlError::Invalidated)))
    {
        return Some(true);
    }
    let visible = state.focused_terminal() == Some(grant.fields().target.address);
    if !matches!(result, Some(Ok(Reply::Input { .. })))
        && crate::terminal_input_authority::interrupt(state, grant)
        && visible
    {
        state.status = match result {
            Some(Err(ControlError::Invalidated)) => crate::terminal_input::CANCELLED_LABEL.into(),
            None => crate::constants::DISCONNECTED_LABEL.into(),
            Some(Err(error)) => error.to_string(),
            Some(Ok(_)) => crate::constants::UNEXPECTED_REPLY_LABEL.into(),
        };
    }
    Some(true)
}

pub(crate) fn control(
    state: &mut State,
    request: &ControlRequest,
    result: &Result<Reply, ControlError>,
    connected: bool,
) -> bool {
    if !matches!(request.command, Command::Input { .. }) {
        return false;
    }
    let matched = match finished(state, request, Some(result)) {
        Some(matched) => matched,
        None => crate::query_replies::completed(state, request, Some(result)).unwrap_or(false),
    };
    if matched && request.generation == state.control_generation && !connected {
        state.disconnected();
    }
    // Unknown, duplicate or altered input receipts cannot fall through to
    // generic writer mutation or manufacture a current connection failure.
    true
}

fn matches_request(expected: &ControlRequest, received: &ControlRequest) -> bool {
    expected.generation == received.generation
        && expected.destination == received.destination
        && expected.command == received.command
        && expected
            .validity
            .as_ref()
            .zip(received.validity.as_ref())
            .is_some_and(|(left, right)| {
                left.same_scope(right) && left.deadline() == right.deadline()
            })
}
