use super::{
    query_support::{Fixture, STEP},
    terminal_input_automatic_loss::{PREFIX, SUFFIX, late_renewal},
    terminal_input_ordering::queued,
};
use crate::{network::Job, polling, terminal_actions};
use std::time::Instant;
use yolo_hub_client::ClientError;
use yolo_hub_client_channels::{ControlError, ControlRequest};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{LeaseGrant, SecretCapability};

fn interrupted() -> Fixture {
    let mut fixture = Fixture::new();
    queued(&mut fixture, PREFIX);
    fixture
        .state
        .writers
        .get_mut(&fixture.target.address)
        .unwrap()
        .expires = Instant::now();
    fixture.state.sync_query_authority();
    fixture
}

fn acquire(fixture: &mut Fixture) -> ControlRequest {
    polling::resume_input(&mut fixture.state, &fixture.jobs, false);
    let Job::Acquire {
        generation,
        origin,
        request,
        validity,
    } = fixture.incoming.try_recv().unwrap()
    else {
        panic!()
    };
    ControlRequest {
        generation: Some(generation),
        destination: origin,
        command: Command::Lease { request },
        validity: Some(validity),
    }
}

#[test]
fn pending_failed_and_unknown_explicit_acquire_cannot_make_old_renewal_resume_input() {
    let mut fixture = interrupted();
    let request = acquire(&mut fixture);
    late_renewal(&mut fixture);
    // The explicit recovery is in flight, so new bytes are admitted against it,
    // but a late renewal of the old writer must not adopt them.
    assert!(terminal_actions::deliver(
        &mut fixture.state,
        &fixture.jobs,
        Ok(SUFFIX.to_vec())
    ));
    assert_eq!(fixture.state.inputs.pregrant.len(), 1);
    assert!(!fixture.state.writers.contains_key(&fixture.target.address));
    fixture.finish(
        request,
        Err(ControlError::Client(ClientError::Remote(
            yolo_hub_terminal_protocol::PublicError::Unauthorized,
        ))),
        true,
    );
    late_renewal(&mut fixture);
    assert!(!fixture.state.writers.contains_key(&fixture.target.address));
    assert!(fixture.state.inputs.pregrant.is_empty());
    let request = acquire(&mut fixture);
    fixture.finish(
        request,
        Err(ControlError::Client(ClientError::Remote(
            yolo_hub_terminal_protocol::PublicError::OutcomeUnknown,
        ))),
        true,
    );
    late_renewal(&mut fixture);
    assert!(!fixture.state.writers.contains_key(&fixture.target.address));
    assert!(terminal_actions::deliver(
        &mut fixture.state,
        &fixture.jobs,
        Ok(SUFFIX.to_vec())
    ));
    assert_eq!(fixture.state.inputs.pregrant.len(), 1);
    fixture.no_jobs();
}

#[test]
fn a_valid_fresh_grant_reply_is_the_boundary_that_resumes_the_interrupted_terminal() {
    let mut fixture = interrupted();
    let request = acquire(&mut fixture);
    late_renewal(&mut fixture);
    assert!(!fixture.state.writers.contains_key(&fixture.target.address));
    let mut fields = *fixture.grant.fields();
    fields.generation += STEP;
    fields.capability = SecretCapability::generate();
    let replacement = LeaseGrant::new(fields).unwrap();
    fixture.finish(request, Ok(Reply::Lease { grant: replacement }), true);
    fixture.ready_size();
    let next = queued(&mut fixture, SUFFIX);
    assert!(next.validity.as_ref().unwrap().is_current());
    assert_eq!(
        fixture.state.writers[&fixture.target.address].grant,
        replacement
    );
}
