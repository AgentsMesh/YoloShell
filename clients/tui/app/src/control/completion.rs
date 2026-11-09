use crate::{State, network::Job};
use std::time::Instant;
use tokio::sync::mpsc;
use yolo_hub_client_channels::{ControlError, ControlRequest};
use yolo_hub_rpc_protocol::Command;
use yolo_hub_terminal_protocol::{LeaseGrant, LeaseRequest, PublicError};
use yolo_tui_terminal_control::{Attempt, Failure};

pub(crate) fn granted(
    state: &mut State,
    jobs: &mpsc::Sender<Job>,
    request: &LeaseRequest,
    grant: LeaseGrant,
) {
    let now = Instant::now();
    let accepted = state
        .control
        .entries
        .get_mut(&request.target.address)
        .is_some_and(|entry| {
            let Some(connection) = state.control_generation else {
                return false;
            };
            let attempt = Attempt {
                target: request.target,
                request_id: request.request_id,
                connection,
            };
            let accepted = entry.policy.succeeded(attempt, now);
            if accepted {
                entry.scope = None;
            }
            accepted
        });
    if accepted {
        crate::lease_events::granted(state, jobs, grant);
    } else {
        release_unaccepted(state, jobs, grant);
    }
}

pub(crate) fn release_unaccepted(state: &mut State, jobs: &mpsc::Sender<Job>, grant: LeaseGrant) {
    // A duplicate receipt cannot release authority already admitted by this view.
    if state
        .writers
        .get(&grant.fields().target.address)
        .is_some_and(|writer| crate::query_state::same_writer(writer.grant, grant))
    {
        return;
    }
    crate::events::enqueue(
        state,
        jobs,
        Job::Call {
            generation: None,
            origin: grant.fields().target.address.hub_id,
            command: Command::Release { grant },
        },
    );
}

pub(crate) fn failed(state: &mut State, request: &ControlRequest, error: &ControlError) {
    if request.generation != state.control_generation {
        return;
    }
    let now = Instant::now();
    match &request.command {
        Command::Lease { request: lease } => {
            let Some(connection) = request.generation else {
                return;
            };
            let initial = state.initial_control.as_ref().is_some_and(|intent| {
                intent.acquire_request_id == Some(lease.request_id)
                    && crate::initial_control::valid_target(state, intent, lease.target)
            });
            if initial {
                let entry = state.control.entry(lease.target);
                entry.policy.request_write(now);
                entry.policy.begin(lease.request_id, connection, now);
            }
            let denied = if let Some(entry) = state.control.entries.get_mut(&lease.target.address) {
                let attempt = Attempt {
                    target: lease.target,
                    request_id: lease.request_id,
                    connection,
                };
                let failure = classify(error, true);
                let accepted = entry.policy.failed(attempt, failure, now);
                if accepted {
                    entry.scope = None;
                }
                accepted && failure == Failure::Denied
            } else {
                false
            };
            // Bytes admitted while waiting for that writer cannot outlive a
            // definitive denial; only a new explicit intent may acquire again.
            if denied {
                crate::terminal_input::clear_pregrant_for(state, lease.target.address);
            }
        }
        Command::Renew { grant } => {
            let address = grant.fields().target.address;
            if state
                .writers
                .get(&address)
                .is_some_and(|writer| crate::query_state::same_writer(writer.grant, *grant))
                && let Some(entry) = state.control.entries.get_mut(&address)
            {
                entry.policy.record_failure(classify(error, false), now);
            }
        }
        _ => {}
    }
}

pub(crate) fn not_sent(state: &mut State, request: &ControlRequest) {
    if let Command::ObserveLease { target } = request.command {
        if super::observe_reply(state, request) {
            state
                .control
                .entry(target)
                .policy
                .record_failure(Failure::Transient, Instant::now());
        }
    }
    if let Command::Lease { .. } = request.command {
        failed(state, request, &ControlError::Invalidated);
    }
}

fn classify(error: &ControlError, acquiring: bool) -> Failure {
    use yolo_hub_client::ClientError;
    match error {
        ControlError::Client(ClientError::Unsupported) => Failure::Denied,
        ControlError::Client(ClientError::Remote(PublicError::Conflict)) if acquiring => {
            Failure::Contended
        }
        ControlError::Client(ClientError::Remote(
            PublicError::Unauthorized
            | PublicError::Unsupported
            | PublicError::UnsupportedSchema
            | PublicError::TargetMismatch
            | PublicError::OriginMismatch,
        )) => Failure::Denied,
        _ => Failure::Transient,
    }
}
