use super::{control_recovery_support::*, query_support::Fixture};
use crate::{control, network::Job, polling};
use yolo_hub_client_channels::ControlRequest;
use yolo_hub_rpc_protocol::Reply;
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
fn takeover_observation_blocks_automatic_acquisition_until_exact_current_reply() {
    let mut fixture = Fixture::new();
    fixture.state.writers.clear();
    let request = observe(&mut fixture);
    polling::renew_leases(&mut fixture.state, &fixture.jobs);
    fixture.no_jobs();
    let lease = holder(&fixture);
    fixture.finish(request, Ok(Reply::LeaseObservation { lease }), true);
    let Job::Acquire {
        request, validity, ..
    } = fixture.incoming.try_recv().unwrap()
    else {
        panic!()
    };
    assert_eq!(
        request.mode,
        LeaseMode::TakeControl {
            observed_generation: fixture.grant.fields().generation,
        }
    );
    assert!(validity.is_current());
    polling::renew_leases(&mut fixture.state, &fixture.jobs);
    fixture.no_jobs();
}

#[test]
fn cancelled_takeover_reply_cannot_upgrade_a_new_safe_acquisition() {
    let mut fixture = Fixture::new();
    fixture.state.writers.clear();
    let old = observe(&mut fixture);
    release_key(&mut fixture);
    control::acquire(&mut fixture.state, &fixture.jobs, fixture.target);
    let current = acquisition(&mut fixture);
    assert!(!old.validity.as_ref().unwrap().is_current());
    let lease = holder(&fixture);
    fixture.finish(old, Ok(Reply::LeaseObservation { lease }), true);
    fixture.no_jobs();
    assert!(current.validity.as_ref().unwrap().is_current());
    let grant = replacement(&fixture);
    fixture.finish(current, Ok(Reply::Lease { grant }), true);
    assert_eq!(fixture.state.writers[&fixture.target.address].grant, grant);
    fixture.no_jobs();
}

#[test]
fn unsent_takeover_observation_cannot_leave_recovery_permanently_pending() {
    let mut fixture = Fixture::new();
    fixture.state.writers.clear();
    let request = observe(&mut fixture);
    crate::channel_events::not_sent(&mut fixture.state, request);
    assert_eq!(
        control::label(&fixture.state, fixture.target.address),
        RESTORING_LABEL
    );
    // Cancelling then requesting normal control cannot revive the old takeover.
    release_key(&mut fixture);
    control::acquire(&mut fixture.state, &fixture.jobs, fixture.target);
    acquisition(&mut fixture);
}
