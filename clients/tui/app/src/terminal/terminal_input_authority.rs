use crate::{State, constants::INPUT_OVERFLOW_LABEL, query_state::same_writer};
use yolo_hub_terminal_protocol::LeaseGrant;
use yolo_tui_terminal_control::Presentation;

pub(crate) const INTERRUPTED_HINT: &str = "Ctrl-b a to resume · check terminal input";
pub(crate) const RESUMING_HINT: &str =
    "Waiting for fresh input control; previous input is not replayed";
const DENIED_HINT: &str = "Check Hub access before retrying";
const OCCUPIED_HINT: &str = "Previous input is not replayed";

pub(crate) fn request_resume(state: &mut State, address: yolo_hub_identity::TerminalAddress) {
    // A sticky overflow retains admitted bytes; resume must let that prefix drain.
    let had_queued_overflow = state.inputs.resume_overflow(address);
    if !had_queued_overflow {
        state.inputs.scope = None;
    }
    state.inputs.failures.remove(&address);
    state.inputs.resume_requested.insert(address);
}

pub(crate) fn blocked(state: &State, address: yolo_hub_identity::TerminalAddress) -> bool {
    state.inputs.interrupted.contains_key(&address)
        && !state.inputs.resume_requested.contains(&address)
}

pub(crate) fn status(state: &State) -> Option<&'static str> {
    if !state
        .client_capability_enabled(yolo_builtin_client_contract::ClientCapability::TerminalControl)
    {
        return None;
    }
    let address = state.focused_terminal()?;
    if matches!(
        crate::terminal_lifecycle::phase(state, address),
        yolo_tui_lifecycle::TerminalPhase::Ended
            | yolo_tui_lifecycle::TerminalPhase::Lost
            | yolo_tui_lifecycle::TerminalPhase::Archived
    ) {
        return None;
    }
    if state.inputs.interrupted.contains_key(&address) {
        Some(if blocked(state, address) {
            INTERRUPTED_HINT
        } else {
            match crate::control::phase(state, address) {
                Some(Presentation::Denied) => DENIED_HINT,
                Some(Presentation::Occupied) => OCCUPIED_HINT,
                _ if state
                    .control
                    .observed_owner(address, state.principal)
                    .is_some() =>
                {
                    OCCUPIED_HINT
                }
                Some(Presentation::Acquiring | Presentation::Recovering) => RESUMING_HINT,
                _ => INTERRUPTED_HINT,
            }
        })
    } else if state.inputs.overflowed.contains(&address) {
        Some(INPUT_OVERFLOW_LABEL)
    } else {
        None
    }
}

pub(crate) fn sync(state: &mut State) {
    crate::terminal_input_stream::sync(state);
    if let Some(scope) = state
        .inputs
        .scope
        .as_ref()
        .filter(|scope| !scope.current(state))
    {
        // Navigation is an explicit change of input destination. Automatic
        // authority/readiness loss must not resume a truncated input stream.
        if scope.same_view(state) && state.inputs.pending_in_scope(scope) {
            state
                .inputs
                .interrupted
                .insert(scope.grant.fields().target.address, scope.grant);
            state
                .inputs
                .resume_requested
                .remove(&scope.grant.fields().target.address);
        }
        state.inputs.scope = None;
    }
    let interrupted = state
        .inputs
        .interrupted
        .values()
        .copied()
        .collect::<Vec<_>>();
    for grant in interrupted {
        let address = grant.fields().target.address;
        let Some(current) = state.writers.get(&address).map(|writer| writer.grant) else {
            continue;
        };
        if same_writer(current, grant) || blocked(state, address) {
            // A renewal confirms the same lease; it cannot authorize input
            // after any missing prefix. Wait for a genuinely new writer grant.
            state.retire_matching_writer(current);
        } else {
            state.inputs.interrupted.remove(&address);
            state.inputs.resume_requested.remove(&address);
        }
    }
}

pub(crate) fn interrupt(state: &mut State, grant: LeaseGrant) -> bool {
    if !state.inputs.matches_stream(state.control_generation, grant) {
        return false;
    }
    state
        .inputs
        .resume_requested
        .remove(&grant.fields().target.address);
    state
        .inputs
        .interrupted
        .insert(grant.fields().target.address, grant);
    state.retire_matching_writer(grant)
}
