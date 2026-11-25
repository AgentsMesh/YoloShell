use crate::{State, constants::CATALOG_REFRESH, network::Job};
use std::time::Instant;
use tokio::sync::mpsc;
use yolo_builtin_client_contract::ClientCapability;
use yolo_hub_client_channels::{ControlError, ControlRequest, RequestScope};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_tui_terminal_control::{ATTEMPT_TIMEOUT, Presentation};

pub(super) struct Probe {
    scope: RequestScope,
    request: ControlRequest,
}

/// Holder metadata is observation only: it can never grant or take writer control.
pub(crate) fn refresh(state: &mut State, jobs: &mpsc::Sender<Job>) {
    if !state.client_capability_enabled(ClientCapability::TerminalControl) {
        return;
    }
    let Some(generation) = state.control_generation else {
        return;
    };
    let now = Instant::now();
    let addresses = state.terminals.keys().copied().collect::<Vec<_>>();
    for address in addresses {
        if state.grant(address, now).is_some()
            || state.is_offline(address)
            || !crate::terminal_lifecycle::phase(state, address).allows_input()
        {
            continue;
        }
        let Some(target) = state
            .terminals
            .get(&address)
            .and_then(|terminal| terminal.fields().runtime)
            .map(|runtime| runtime.target)
        else {
            continue;
        };
        let entry = state.control.entry(target);
        if now < entry.next_probe
            || entry.takeover.is_some()
            || entry.scope.is_some()
            || !matches!(
                entry.policy.phase(now, false),
                Presentation::ReadOnly | Presentation::Occupied
            )
        {
            continue;
        }
        if entry.probe.as_ref().is_some_and(|probe| {
            probe
                .request
                .validity
                .as_ref()
                .is_some_and(|validity| validity.is_current())
        }) {
            continue;
        }
        entry.probe = None;
        entry.next_probe = now + CATALOG_REFRESH;
        let scope = RequestScope::default();
        let request = ControlRequest {
            generation: Some(generation),
            destination: address.hub_id,
            command: Command::ObserveLease { target },
            validity: Some(scope.validity_until(now + ATTEMPT_TIMEOUT)),
        };
        if jobs.try_send(Job::Scoped(request.clone())).is_ok() {
            entry.probe = Some(Probe { scope, request });
        }
    }
}

/// Consume only our exact request. Takeover observations use their separate path.
pub(crate) fn completed(
    state: &mut State,
    request: &ControlRequest,
    result: Option<&Result<Reply, ControlError>>,
) -> bool {
    let Command::ObserveLease { target } = request.command else {
        return false;
    };
    let Some(entry) = state.control.entries.get_mut(&target.address) else {
        return false;
    };
    let Some(probe) = entry.probe.as_ref() else {
        return false;
    };
    let matches = request.generation == probe.request.generation
        && request.destination == probe.request.destination
        && request.command == probe.request.command
        && request
            .validity
            .as_ref()
            .zip(probe.request.validity.as_ref())
            .is_some_and(|(received, expected)| {
                received.same_scope(expected) && received.deadline() == expected.deadline()
            });
    if !matches {
        return false;
    }
    let current = request.generation == state.control_generation
        && entry.policy.target() == target
        && request.validity.as_ref().is_some_and(|validity| {
            validity.is_current()
                && validity.same_scope(&probe.scope.validity_until(validity.deadline()))
        });
    entry.probe = None;
    if !current {
        return true;
    }
    if state.grant(target.address, Instant::now()).is_none()
        && crate::observation_events::current(state, target)
    {
        match result {
            Some(Ok(Reply::LeaseObservation { lease })) => {
                state.control.record_observation(target, lease.as_ref());
            }
            Some(Err(ControlError::Client(
                yolo_hub_client::ClientError::Unsupported
                | yolo_hub_client::ClientError::Remote(
                    yolo_hub_terminal_protocol::PublicError::Unsupported,
                ),
            ))) => {
                let entry = state.control.entry(target);
                entry.observation = None;
                entry
                    .policy
                    .record_failure(yolo_tui_terminal_control::Failure::Denied, Instant::now());
            }
            _ => state.control.clear_observation(target.address),
        }
    }
    true
}
