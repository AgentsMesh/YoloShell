use super::query_support::Fixture;
use crate::{control, network::Job};
use std::time::{SystemTime, UNIX_EPOCH};
use yolo_hub_client_channels::ControlRequest;
use yolo_hub_rpc_protocol::Reply;
use yolo_hub_terminal_protocol::{LeaseObservation, LeaseObservationFields};

const HOLDER: &str = "observed-ui";
const OBSERVATION_LIFETIME_MS: i64 = 60_000;

fn probe(fixture: &mut Fixture) -> ControlRequest {
    fixture.state.writers.clear();
    fixture.state.control.observe(fixture.target);
    control::refresh_observations(&mut fixture.state, &fixture.jobs);
    let Job::Scoped(request) = fixture.incoming.try_recv().unwrap() else {
        panic!("expected holder observation")
    };
    request
}

fn reply(fixture: &Fixture) -> Reply {
    Reply::LeaseObservation {
        lease: Some(
            LeaseObservation::new(LeaseObservationFields {
                target: fixture.target,
                generation: fixture.grant.fields().generation,
                holder_id: HOLDER.into(),
                controller_instance_id: None,
                expires_at_ms: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as i64
                    + OBSERVATION_LIFETIME_MS,
            })
            .unwrap(),
        ),
    }
}

#[test]
fn read_only_observation_displays_holder_without_requesting_a_writer() {
    let mut fixture = Fixture::new();
    let request = probe(&mut fixture);
    let result = reply(&fixture);
    fixture.finish(request, Ok(result), true);
    assert_eq!(
        fixture
            .state
            .control
            .observed_holder(fixture.target.address),
        Some(HOLDER)
    );
    assert!(fixture.state.writers.is_empty());
    assert!(!fixture.state.control.permits(fixture.target));
    fixture.no_jobs();
    control::refresh_observations(&mut fixture.state, &fixture.jobs);
    fixture.no_jobs();
}

#[test]
fn stale_or_duplicate_probe_cannot_turn_observation_into_takeover() {
    let mut fixture = Fixture::new();
    let old = probe(&mut fixture);
    let current = probe(&mut fixture);
    let result = reply(&fixture);
    fixture.finish(old, Ok(result.clone()), true);
    assert!(
        fixture
            .state
            .control
            .observed_holder(fixture.target.address)
            .is_none()
    );
    fixture.no_jobs();
    fixture.finish(current.clone(), Ok(result.clone()), true);
    fixture.finish(current, Ok(result), true);
    fixture.no_jobs();
    assert!(fixture.state.writers.is_empty());
}

#[test]
fn disconnected_or_tampered_probe_cannot_update_holder() {
    let mut fixture = Fixture::new();
    let request = probe(&mut fixture);
    let mut changed = request.clone();
    changed.generation = Some(uuid::Uuid::new_v4());
    let result = reply(&fixture);
    fixture.finish(changed, Ok(result.clone()), true);
    assert!(
        fixture
            .state
            .control
            .observed_holder(fixture.target.address)
            .is_none()
    );
    fixture.state.disconnected();
    fixture.finish(request, Ok(result), true);
    assert!(
        fixture
            .state
            .control
            .observed_holder(fixture.target.address)
            .is_none()
    );
    fixture.no_jobs();
}

#[test]
fn failed_observation_never_revokes_an_existing_writer() {
    let mut fixture = Fixture::new();
    let request = probe(&mut fixture);
    fixture
        .state
        .record_grant(fixture.grant, std::time::Instant::now());
    fixture.finish(
        request,
        Err(yolo_hub_client_channels::ControlError::Invalidated),
        true,
    );
    assert_eq!(
        fixture.state.writers[&fixture.target.address].grant,
        fixture.grant
    );
    fixture.no_jobs();
}

#[test]
fn unbound_catalogue_terminal_is_observed_without_acquiring_it() {
    let mut fixture = Fixture::new();
    fixture.state.bind_focused(None).unwrap();
    let request = probe(&mut fixture);
    control::reconcile(&mut fixture.state, &fixture.jobs);
    let result = reply(&fixture);
    fixture.finish(request, Ok(result), true);
    assert_eq!(
        fixture
            .state
            .control
            .observed_holder(fixture.target.address),
        Some(HOLDER)
    );
    fixture.no_jobs();
}

#[test]
fn takeover_retires_the_older_passive_observation() {
    let mut fixture = Fixture::new();
    let old = probe(&mut fixture);
    control::takeover(&mut fixture.state, &fixture.jobs, fixture.target);
    assert!(matches!(fixture.incoming.try_recv(), Ok(Job::Scoped(_))));
    let result = reply(&fixture);
    fixture.finish(old, Ok(result), true);
    assert!(
        fixture
            .state
            .control
            .observed_holder(fixture.target.address)
            .is_none()
    );
    fixture.no_jobs();
}
