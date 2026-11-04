use crate::{State, events::enqueue, network::Job};
use std::time::Instant;
use tokio::sync::mpsc;
use yolo_hub_identity::{RuntimeTarget, TerminalAddress};
use yolo_hub_rpc_protocol::Command;
use yolo_hub_terminal_protocol::{LeaseGrant, LeaseMode, LeaseObservation};

pub fn observed(
    state: &mut State,
    jobs: &mpsc::Sender<Job>,
    target: RuntimeTarget,
    lease: Option<LeaseObservation>,
) {
    if !crate::observation_events::current(state, target)
        || !bound(state, target.address)
        || !state.control.permits(target)
        || !crate::terminal_lifecycle::phase(state, target.address).allows_input()
    {
        state.control.clear_observation(target.address);
        return;
    }
    state.control.record_observation(target, lease.as_ref());
    let mode = lease.map_or(LeaseMode::Acquire {}, |lease| LeaseMode::TakeControl {
        observed_generation: lease.fields().generation,
    });
    crate::control::dispatch(state, jobs, target, mode);
}

pub fn granted(state: &mut State, jobs: &mpsc::Sender<Job>, grant: LeaseGrant) {
    let target = grant.fields().target;
    if !crate::observation_events::current(state, target) {
        return;
    }
    state.clear_renewal_pending(target.address);
    if crate::terminal_input_authority::blocked(state, target.address)
        && state
            .inputs
            .interrupted
            .get(&target.address)
            .is_some_and(|interrupted| crate::query_state::same_writer(*interrupted, grant))
    {
        return;
    }
    if !bound(state, target.address)
        || crate::terminal_input_authority::blocked(state, target.address)
        || !state.control.permits(target)
        || !crate::terminal_lifecycle::phase(state, target.address).allows_input()
    {
        // A tab may close while acquisition or renewal is in flight.
        enqueue(
            state,
            jobs,
            Job::Call {
                generation: None,
                origin: target.address.hub_id,
                command: Command::Release { grant },
            },
        );
        return;
    }
    state.record_grant(grant, Instant::now());
    crate::terminal_input::flush_pregrant(state, jobs, target.address, grant);
    if matches!(
        state.status.as_str(),
        crate::constants::INPUT_WAITING_LABEL | crate::constants::CONTROL_LOST_LABEL
    ) {
        state.status = crate::constants::CONNECTED_LABEL.into();
    }
}

pub fn renewed(
    state: &mut State,
    jobs: &mpsc::Sender<Job>,
    previous: LeaseGrant,
    grant: LeaseGrant,
) {
    let address = grant.fields().target.address;
    let current = state.writers.get(&address).is_some_and(|writer| {
        crate::query_state::same_writer(writer.grant, previous)
            && crate::query_state::same_writer(previous, grant)
    });
    if current {
        granted(state, jobs, grant);
    }
}

fn bound(state: &State, address: TerminalAddress) -> bool {
    state.bound_terminals().contains(&address)
}

/// Another client holding input is a normal observation result for safe Acquire
/// and for a lost Renew/Input lease; it reads as read-only, not as a failure.
/// Unknown outcomes and explicit takeover failures keep their diagnostic meaning.
pub(crate) fn failure_message(
    command: &Command,
    error: &yolo_hub_client_channels::ControlError,
) -> String {
    let conflict = matches!(
        error,
        yolo_hub_client_channels::ControlError::Client(yolo_hub_client::ClientError::Remote(
            yolo_hub_terminal_protocol::PublicError::Conflict
        ))
    );
    if conflict {
        match command {
            Command::Lease { request } if request.mode == LeaseMode::Acquire {} => {
                return crate::constants::READ_ONLY_LABEL.into();
            }
            Command::Renew { .. } | Command::Input { .. } => {
                return crate::constants::CONTROL_LOST_LABEL.into();
            }
            _ => {}
        }
    }
    error.to_string()
}
