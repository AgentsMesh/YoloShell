use super::{control_recovery_support::*, query_support::Fixture, terminal_input_ordering::queued};
use crate::{
    control, events,
    network::{ObservationUpdate, Update},
    polling, terminal_actions,
};
use std::time::Instant;
use yolo_hub_client_channels::{ControlError, ControlEvent};
use yolo_hub_rpc_protocol::Reply;

#[test]
fn connection_recovers_write_intent_and_accepts_new_input_without_picker_navigation() {
    let mut fixture = Fixture::new();
    let generation = fixture.state.control_generation;
    elapsed_retry(&mut fixture);
    events::apply(
        &mut fixture.state,
        &fixture.jobs,
        Update::Control(ControlEvent::Unreachable {
            generation,
            message: crate::constants::DISCONNECTED_LABEL.into(),
        }),
    );
    assert!(fixture.state.writers.is_empty());
    assert!(fixture.state.control.permits(fixture.target));
    polling::renew_leases(&mut fixture.state, &fixture.jobs);
    fixture.no_jobs();
    ready(&mut fixture);
    polling::renew_leases(&mut fixture.state, &fixture.jobs);
    let attempt = acquisition(&mut fixture);
    assert_ne!(attempt.generation, generation);
    fixture.no_jobs();
    let grant = replacement(&fixture);
    fixture.finish(attempt, Ok(Reply::Lease { grant }), true);
    fixture.ready_size();
    assert_eq!(
        fixture.state.grant(fixture.target.address, Instant::now()),
        Some(grant)
    );
    fixture.no_jobs();
    let input = queued(&mut fixture, NEW_INPUT);
    assert_eq!(super::query_support::input(&input).fields().data, NEW_INPUT);
}

#[test]
fn idle_output_disconnect_recovers_readiness_without_reacquiring_the_healthy_writer() {
    let mut fixture = Fixture::new();
    fixture.scoped(ObservationUpdate::Closed { message: None });
    assert!(fixture.state.is_offline(fixture.target.address));
    assert!(!fixture.state.input_ready(fixture.target.address));
    assert!(!fixture.state.keyboard_ready(fixture.target.address));
    assert!(fixture.state.input_blocked(fixture.target.address));
    assert_eq!(
        fixture.state.writers[&fixture.target.address].grant,
        fixture.grant
    );
    assert!(!terminal_actions::deliver(
        &mut fixture.state,
        &fixture.jobs,
        Ok(NEW_INPUT.to_vec())
    ));
    polling::renew_leases(&mut fixture.state, &fixture.jobs);
    fixture.no_jobs();
    reconnect_output(&mut fixture);
    assert!(!fixture.state.is_offline(fixture.target.address));
    assert!(fixture.state.input_ready(fixture.target.address));
    polling::renew_leases(&mut fixture.state, &fixture.jobs);
    fixture.no_jobs();
    let input = queued(&mut fixture, NEW_INPUT);
    assert_eq!(
        super::query_support::input(&input).fields().grant,
        fixture.grant
    );
}

#[test]
fn output_loss_with_unconfirmed_approval_input_requires_a_new_grant_and_never_replays() {
    let mut fixture = Fixture::new();
    let old_input = queued(&mut fixture, NEW_INPUT);
    elapsed_retry(&mut fixture);
    fixture.scoped(ObservationUpdate::Closed { message: None });
    assert!(!old_input.validity.as_ref().unwrap().is_current());
    assert!(fixture.state.writers.is_empty());
    polling::renew_leases(&mut fixture.state, &fixture.jobs);
    fixture.no_jobs();
    reconnect_output(&mut fixture);
    polling::renew_leases(&mut fixture.state, &fixture.jobs);
    fixture.no_jobs();
    polling::resume_input(&mut fixture.state, &fixture.jobs, false);
    let attempt = acquisition(&mut fixture);
    let grant = replacement(&fixture);
    fixture.finish(attempt, Ok(Reply::Lease { grant }), true);
    fixture.ready_size();
    fixture.finish(old_input, Err(ControlError::Invalidated), true);
    assert!(fixture.state.inputs.pending.is_empty());
    assert_eq!(fixture.state.writers[&fixture.target.address].grant, grant);
    fixture.no_jobs();
    let next = queued(&mut fixture, NEW_INPUT);
    assert_eq!(super::query_support::input(&next).fields().grant, grant);
}

#[test]
fn pending_acquisition_is_coalesced_across_ticks_and_repeated_open_intent() {
    let mut fixture = Fixture::new();
    fixture.state.writers.clear();
    control::acquire(&mut fixture.state, &fixture.jobs, fixture.target);
    let current = acquisition(&mut fixture);
    for _ in [(), (), ()] {
        control::acquire(&mut fixture.state, &fixture.jobs, fixture.target);
        polling::renew_leases(&mut fixture.state, &fixture.jobs);
    }
    fixture.no_jobs();
    assert!(current.validity.unwrap().is_current());
}
