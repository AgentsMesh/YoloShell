use super::{
    control_recovery_support::replacement,
    query_support::Fixture,
    terminal_input_navigation_support::{PREFIX, SUFFIX},
    terminal_input_ordering::queued,
};
use crate::{network::Job, polling, terminal_input_authority};
use yolo_hub_client_channels::{ControlError, ControlRequest};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{LeaseMode, LeaseObservation, LeaseObservationFields};

#[test]
fn a_takeover_grant_retires_input_queued_while_the_takeover_was_pending() {
    let mut fixture = Fixture::new();
    let before_takeover = queued(&mut fixture, PREFIX);
    polling::resume_input(&mut fixture.state, &fixture.jobs, true);
    fixture.state.sync_query_authority();
    let Job::Scoped(observation) = fixture.incoming.try_recv().unwrap() else {
        panic!();
    };
    assert!(matches!(
        observation.command,
        Command::ObserveLease { target } if target == fixture.target
    ));
    let lease = LeaseObservation::new(LeaseObservationFields {
        target: fixture.target,
        generation: fixture.grant.fields().generation,
        holder_id: super::fixtures::SESSION_NAME.into(),
        controller_instance_id: None,
        expires_at_ms: super::fixtures::EXPIRES_MS,
    })
    .unwrap();
    fixture.finish(
        observation,
        Ok(Reply::LeaseObservation { lease: Some(lease) }),
        true,
    );
    let Job::Acquire {
        generation,
        origin,
        request,
        validity,
    } = fixture.incoming.try_recv().unwrap()
    else {
        panic!();
    };
    assert_eq!(
        request.mode,
        LeaseMode::TakeControl {
            observed_generation: fixture.grant.fields().generation
        }
    );
    let takeover = ControlRequest {
        generation: Some(generation),
        destination: origin,
        command: Command::Lease { request },
        validity: Some(validity),
    };
    // The old writer remains usable while the requested takeover is awaiting
    // its exact response. This real keyboard event creates another view scope.
    let during_takeover = queued(&mut fixture, SUFFIX);
    assert!(during_takeover.validity.as_ref().unwrap().is_current());
    let fresh = replacement(&fixture);
    fixture.finish(takeover, Ok(Reply::Lease { grant: fresh }), true);
    assert_eq!(fixture.state.writers[&fixture.target.address].grant, fresh);
    assert!(fixture.state.inputs.scope.is_none());
    assert!(!before_takeover.validity.as_ref().unwrap().is_current());
    assert!(!during_takeover.validity.as_ref().unwrap().is_current());
    assert!(!terminal_input_authority::blocked(
        &fixture.state,
        fixture.target.address
    ));

    fixture.ready_size();
    let after_takeover = queued(&mut fixture, PREFIX);
    for obsolete in [before_takeover, during_takeover] {
        fixture.finish(obsolete, Err(ControlError::Invalidated), true);
        assert!(after_takeover.validity.as_ref().unwrap().is_current());
        assert_eq!(fixture.state.writers[&fixture.target.address].grant, fresh);
    }
    fixture.finish(after_takeover, Ok(Reply::Input { duplicate: false }), true);
    assert!(fixture.state.inputs.pending.is_empty());
    fixture.no_jobs();
}
