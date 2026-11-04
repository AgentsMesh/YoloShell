use super::{control_recovery_support::*, query_support::Fixture};
use crate::{
    control, events,
    network::{Job, Update},
};
use uuid::Uuid;
use yolo_hub_client_channels::ControlError;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::LeaseMode;

const SINGLE_SLOT: usize = 1;

#[test]
fn full_queue_preserves_recoverable_intent_without_replaying_takeover() {
    for takeover in [false, true] {
        let mut fixture = Fixture::with_capacity(SINGLE_SLOT);
        fixture.state.writers.clear();
        fixture
            .jobs
            .try_send(Job::Refresh {
                request_id: Uuid::new_v4(),
            })
            .ok()
            .unwrap();
        if takeover {
            control::takeover(&mut fixture.state, &fixture.jobs, fixture.target);
        } else {
            control::acquire(&mut fixture.state, &fixture.jobs, fixture.target);
        }
        assert_eq!(fixture.state.status, crate::constants::QUEUE_FULL_LABEL);
        assert!(matches!(
            fixture.incoming.try_recv().unwrap(),
            Job::Refresh { .. }
        ));
        fixture.no_jobs();
        assert!(fixture.state.writers.is_empty());
        control::acquire(&mut fixture.state, &fixture.jobs, fixture.target);
        acquisition(&mut fixture);
        fixture.no_jobs();
    }
}

#[test]
fn asynchronous_unsent_acquisition_can_retry_without_accepting_the_cancelled_reply() {
    let mut fixture = Fixture::new();
    fixture.state.writers.clear();
    control::acquire(&mut fixture.state, &fixture.jobs, fixture.target);
    let old = acquisition(&mut fixture);
    events::apply(
        &mut fixture.state,
        &fixture.jobs,
        Update::NotSent {
            request: old.clone(),
        },
    );
    assert!(!old.validity.as_ref().unwrap().is_current());
    assert_eq!(
        control::label(&fixture.state, fixture.target.address),
        RESTORING_LABEL
    );
    control::acquire(&mut fixture.state, &fixture.jobs, fixture.target);
    let current = acquisition(&mut fixture);
    let stale = fixture.grant;
    fixture.finish(old, Ok(Reply::Lease { grant: stale }), true);
    released(&mut fixture, stale);
    assert!(fixture.state.writers.is_empty());
    assert!(current.validity.as_ref().unwrap().is_current());
}

#[test]
fn disconnected_dispatch_and_completion_cannot_create_input_authority() {
    let mut fixture = Fixture::new();
    fixture.state.writers.clear();
    control::acquire(&mut fixture.state, &fixture.jobs, fixture.target);
    let mut pending = acquisition(&mut fixture);
    fixture.state.disconnected();
    control::dispatch(
        &mut fixture.state,
        &fixture.jobs,
        fixture.target,
        LeaseMode::Acquire {},
    );
    control::takeover(&mut fixture.state, &fixture.jobs, fixture.target);
    fixture.no_jobs();
    control::failed(&mut fixture.state, &pending, &ControlError::NotConnected);
    pending.generation = None;
    control::failed(&mut fixture.state, &pending, &ControlError::NotConnected);
    let Command::Lease { request } = pending.command else {
        panic!()
    };
    let grant = replacement(&fixture);
    control::granted(&mut fixture.state, &fixture.jobs, &request, grant);
    fixture.no_jobs();
    assert!(fixture.state.writers.is_empty());
    assert_eq!(
        control::label(&fixture.state, fixture.target.address),
        RESTORING_LABEL
    );
}

#[test]
fn repeated_dispatch_cannot_replace_an_inflight_acquisition() {
    let mut fixture = Fixture::new();
    fixture.state.writers.clear();
    control::acquire(&mut fixture.state, &fixture.jobs, fixture.target);
    let current = acquisition(&mut fixture);
    control::dispatch(
        &mut fixture.state,
        &fixture.jobs,
        fixture.target,
        LeaseMode::Acquire {},
    );
    fixture.no_jobs();
    assert!(current.validity.as_ref().unwrap().is_current());
    let grant = replacement(&fixture);
    fixture.finish(current, Ok(Reply::Lease { grant }), true);
    assert_eq!(fixture.state.writers[&fixture.target.address].grant, grant);
}
