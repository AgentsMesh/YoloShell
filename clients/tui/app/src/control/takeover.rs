use crate::{State, network::Job};
use std::time::Instant;
use tokio::sync::mpsc;
use yolo_builtin_client_contract::ClientCapability;
use yolo_hub_client_channels::{ControlRequest, RequestScope};
use yolo_hub_identity::RuntimeTarget;
use yolo_hub_rpc_protocol::Command;
use yolo_tui_terminal_control::ATTEMPT_TIMEOUT;

pub(crate) fn takeover(state: &mut State, jobs: &mpsc::Sender<Job>, target: RuntimeTarget) {
    if !crate::client_capabilities::require(
        state,
        ClientCapability::TerminalControl,
        crate::client_capabilities::TERMINAL_CONTROL_UNAVAILABLE,
    ) {
        return;
    }
    let Some(generation) = state.control_generation else {
        return;
    };
    let now = Instant::now();
    let entry = state.control.entry(target);
    entry.policy.observe();
    entry.policy.request_write(now);
    entry.scope = None;
    entry.probe = None;
    let scope = RequestScope::default();
    let request = ControlRequest {
        generation: Some(generation),
        destination: target.address.hub_id,
        command: Command::ObserveLease { target },
        validity: Some(scope.validity_until(now + ATTEMPT_TIMEOUT)),
    };
    if jobs.try_send(Job::Scoped(request)).is_ok() {
        entry.takeover = Some((scope, now + ATTEMPT_TIMEOUT));
    } else {
        state.status = crate::constants::QUEUE_FULL_LABEL.into();
    }
}

pub(crate) fn observe_reply(state: &mut State, request: &ControlRequest) -> bool {
    let Command::ObserveLease { target } = request.command else {
        return true;
    };
    let Some(entry) = state.control.entries.get_mut(&target.address) else {
        return false;
    };
    let matches = request.generation == state.control_generation
        && entry.policy.target() == target
        && request.validity.as_ref().is_some_and(|validity| {
            validity.is_current()
                && entry.takeover.as_ref().is_some_and(|(scope, _)| {
                    validity.same_scope(&scope.validity_until(validity.deadline()))
                })
        });
    if matches {
        entry.takeover = None;
    }
    matches
}
