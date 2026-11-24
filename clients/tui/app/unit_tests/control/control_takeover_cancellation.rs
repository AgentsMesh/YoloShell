use super::{control_recovery_support::*, query_support::Fixture};
use crate::{control, network::Job};
use uuid::Uuid;
use yolo_hub_client_channels::ControlRequest;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{LeaseMode, LeaseObservation, LeaseObservationFields};

fn observe(fixture: &mut Fixture) -> ControlRequest {
    control::takeover(&mut fixture.state, &fixture.jobs, fixture.target);
    let Job::Scoped(request) = fixture.incoming.try_recv().unwrap() else {
        panic!()
    };
    request
}

fn holder(fixture: &Fixture) -> Option<LeaseObservation> {
    Some(
        LeaseObservation::new(LeaseObservationFields {
            target: fixture.target,
            generation: fixture.grant.fields().generation,
            holder_id: super::fixtures::SESSION_NAME.into(),
            controller_instance_id: None,
            expires_at_ms: super::fixtures::EXPIRES_MS,
        })
        .unwrap(),
    )
}

#[test]
fn safe_acquire_cancels_a_pending_takeover_observation_without_reviving_it() {
    let mut fixture = Fixture::new();
    fixture.state.writers.clear();
    let old = observe(&mut fixture);
    control::acquire(&mut fixture.state, &fixture.jobs, fixture.target);
    let safe = acquisition(&mut fixture);
    assert!(!old.validity.as_ref().unwrap().is_current());
    control::not_sent(&mut fixture.state, &old);
    let lease = holder(&fixture);
    fixture.finish(old, Ok(Reply::LeaseObservation { lease }), true);
    fixture.no_jobs();
    assert!(safe.validity.as_ref().unwrap().is_current());
}

#[test]
fn safe_acquire_cancels_an_admitted_takeover_and_releases_any_late_grant() {
    let mut fixture = Fixture::new();
    fixture.state.writers.clear();
    let observed = observe(&mut fixture);
    let lease = holder(&fixture);
    fixture.finish(observed, Ok(Reply::LeaseObservation { lease }), true);
    let Job::Acquire {
        generation,
        origin,
        request,
        validity,
    } = fixture.incoming.try_recv().unwrap()
    else {
        panic!()
    };
    assert!(matches!(request.mode, LeaseMode::TakeControl { .. }));
    let old = ControlRequest {
        generation: Some(generation),
        destination: origin,
        command: Command::Lease { request },
        validity: Some(validity),
    };
    control::acquire(&mut fixture.state, &fixture.jobs, fixture.target);
    let safe = acquisition(&mut fixture);
    assert!(!old.validity.as_ref().unwrap().is_current());
    let stale = fixture.grant;
    fixture.finish(old, Ok(Reply::Lease { grant: stale }), true);
    released(&mut fixture, stale);
    assert!(safe.validity.as_ref().unwrap().is_current());
    assert!(fixture.state.writers.is_empty());
}

#[test]
fn unrelated_or_replaced_observer_identity_cannot_authorize_takeover() {
    let mut fixture = Fixture::new();
    fixture.state.writers.clear();
    let current = observe(&mut fixture);
    let mut old = current.clone();
    old.generation = Some(Uuid::new_v4());
    assert!(!control::observe_reply(&mut fixture.state, &old));
    let mut other = fixture.target;
    other.runtime_epoch = yolo_hub_identity::RuntimeEpoch::new();
    old = current.clone();
    old.command = Command::ObserveLease { target: other };
    assert!(!control::observe_reply(&mut fixture.state, &old));
    other.address.terminal_id = yolo_hub_identity::TerminalId::new();
    old.command = Command::ObserveLease { target: other };
    assert!(!control::observe_reply(&mut fixture.state, &old));
    assert!(current.validity.as_ref().unwrap().is_current());
    assert!(control::observe_reply(&mut fixture.state, &current));
    fixture.no_jobs();
}
