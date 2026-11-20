use super::fixtures;
use crate::{
    events::apply,
    network::{Job, Update},
};
use tokio::sync::mpsc;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{LeaseMode, LeaseRequest};

const QUEUE_CAPACITY: usize = 1;

#[test]
fn a_late_acquisition_or_renewal_cannot_restore_an_unbound_terminals_writer() {
    let session = fixtures::session();
    let grant = fixtures::grant(&session);
    let target = grant.fields().target;
    let commands = [
        Command::Lease {
            request: LeaseRequest {
                request_id: uuid::Uuid::new_v4(),
                target,
                principal: grant.fields().principal,
                mode: LeaseMode::Acquire {},
            },
        },
        Command::Renew { grant },
    ];
    for command in commands {
        let acquisition = matches!(command, Command::Lease { .. });
        let mut state = crate::client_capability_support::state();
        state.replace_terminals(vec![session.clone()]);
        state.mark_renewal_pending(target.address);
        let (jobs, mut received) = mpsc::channel(QUEUE_CAPACITY);
        apply(
            &mut state,
            &jobs,
            Update::Reply {
                command,
                reply: Reply::Lease { grant },
            },
        );
        assert!(state.writers.is_empty());
        if acquisition {
            assert!(matches!(received.try_recv().unwrap(), Job::Call {
            command: Command::Release { grant: released }, ..
            } if released == grant));
        } else {
            assert!(received.try_recv().is_err());
        }
    }
}

#[test]
fn delayed_takeover_observation_is_cancelled_when_no_pane_is_bound() {
    let session = fixtures::session();
    let target = session.fields().runtime.unwrap().target;
    let mut state = crate::client_capability_support::state();
    state.replace_terminals(vec![session]);
    let (jobs, mut received) = mpsc::channel(QUEUE_CAPACITY);
    apply(
        &mut state,
        &jobs,
        Update::Reply {
            command: Command::ObserveLease { target },
            reply: Reply::LeaseObservation { lease: None },
        },
    );
    assert!(received.try_recv().is_err());
}

#[test]
fn a_bound_current_session_still_receives_its_grant() {
    let session = fixtures::session();
    let grant = fixtures::grant(&session);
    let address = grant.fields().target.address;
    let mut state = crate::client_capability_support::state();
    state.replace_terminals(vec![session]);
    state.bind_focused(Some(address)).unwrap();
    state.record_grant(grant, std::time::Instant::now());
    let (jobs, mut received) = mpsc::channel(QUEUE_CAPACITY);
    apply(
        &mut state,
        &jobs,
        Update::Reply {
            command: Command::Renew { grant },
            reply: Reply::Lease { grant },
        },
    );
    assert_eq!(state.writers[&address].grant, grant);
    assert!(received.try_recv().is_err());
}

#[test]
fn queue_pressure_cannot_restore_a_late_hidden_grant() {
    let session = fixtures::session();
    let grant = fixtures::grant(&session);
    let mut state = crate::client_capability_support::state();
    state.replace_terminals(vec![session]);
    let (jobs, _received) = mpsc::channel(QUEUE_CAPACITY);
    jobs.try_send(Job::Refresh {
        request_id: uuid::Uuid::new_v4(),
    })
    .ok()
    .unwrap();
    apply(
        &mut state,
        &jobs,
        Update::Reply {
            command: Command::Renew { grant },
            reply: Reply::Lease { grant },
        },
    );
    assert!(state.writers.is_empty());
}

#[test]
fn bound_takeover_uses_the_observed_generation_and_empty_lease_uses_acquire() {
    use yolo_hub_terminal_protocol::{
        FIRST_LEASE_GENERATION, LeaseObservation, LeaseObservationFields,
    };
    let session = fixtures::session();
    let target = session.fields().runtime.unwrap().target;
    let mut state = crate::client_capability_support::state();
    state.control_generation = Some(uuid::Uuid::new_v4());
    state.replace_terminals(vec![session]);
    state.bind_focused(Some(target.address)).unwrap();
    let observation = LeaseObservation::new(LeaseObservationFields {
        target,
        generation: FIRST_LEASE_GENERATION,
        holder_id: fixtures::SESSION_NAME.into(),
        controller_instance_id: None,
        expires_at_ms: fixtures::EXPIRES_MS,
    })
    .unwrap();
    let (jobs, mut received) = mpsc::channel(QUEUE_CAPACITY);
    for (lease, expected) in [
        (None, LeaseMode::Acquire {}),
        (
            Some(observation),
            LeaseMode::TakeControl {
                observed_generation: FIRST_LEASE_GENERATION,
            },
        ),
    ] {
        state.control.observe(target);
        state.control.adopt(target, std::time::Instant::now());
        apply(
            &mut state,
            &jobs,
            Update::Reply {
                command: Command::ObserveLease { target },
                reply: Reply::LeaseObservation { lease },
            },
        );
        assert!(matches!(received.try_recv().unwrap(), Job::Acquire {
            origin, request, ..
        } if origin == target.address.hub_id && request.target == target && request.mode == expected));
    }
}
