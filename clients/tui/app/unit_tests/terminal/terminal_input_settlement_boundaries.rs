use super::{
    query_support::Fixture,
    terminal_input_navigation_support::{PREFIX, SUFFIX, unknown},
    terminal_input_ordering::queued,
};
use crate::{terminal_input_authority, terminal_input_stream};
use std::time::Instant;
use yolo_hub_rpc_protocol::Reply;

#[test]
fn a_settled_writer_replacement_discards_the_old_scope_without_manufacturing_interruption() {
    let mut fixture = Fixture::new();
    let first = queued(&mut fixture, PREFIX);
    fixture.finish(first.clone(), Ok(Reply::Input { duplicate: false }), true);
    assert!(fixture.state.inputs.scope.is_some());

    let replacement = fixture.replace_writer();

    assert!(fixture.state.inputs.streams.is_empty());
    assert!(fixture.state.inputs.scope.is_none());
    assert!(!first.validity.as_ref().unwrap().is_current());
    assert!(!terminal_input_authority::blocked(
        &fixture.state,
        fixture.target.address
    ));
    assert_eq!(
        fixture.state.writers[&fixture.target.address].grant,
        replacement
    );
    let current = queued(&mut fixture, SUFFIX);
    assert!(current.validity.as_ref().unwrap().is_current());
    fixture.finish(current, Ok(Reply::Input { duplicate: false }), true);
    assert!(fixture.state.inputs.pending.is_empty());
    fixture.no_jobs();
}

#[test]
fn reconnecting_with_the_same_lease_cannot_erase_the_previous_connection_input_obligation() {
    let mut fixture = Fixture::new();
    let original = queued(&mut fixture, PREFIX);
    let generation = fixture.state.control_generation.unwrap();
    fixture.state.disconnected();
    fixture.state.control_generation = Some(uuid::Uuid::new_v4());

    fixture.state.record_grant(fixture.grant, Instant::now());

    assert_eq!(
        fixture.state.inputs.streams[&fixture.target.address].generation,
        generation
    );
    assert!(!original.validity.as_ref().unwrap().is_current());
    assert!(fixture.state.writers.is_empty());
    assert!(terminal_input_authority::blocked(
        &fixture.state,
        fixture.target.address
    ));
    fixture.finish(original, unknown(), true);
    assert!(fixture.state.inputs.pending.is_empty());
    assert!(terminal_input_authority::blocked(
        &fixture.state,
        fixture.target.address
    ));
    fixture.no_jobs();
}

#[test]
fn an_obsolete_cancellation_cannot_interrupt_a_settled_replacement_writer() {
    let mut fixture = Fixture::new();
    let first = queued(&mut fixture, PREFIX);
    fixture.finish(first, Ok(Reply::Input { duplicate: false }), true);
    let replacement = fixture.replace_writer();
    let current = queued(&mut fixture, SUFFIX);
    let status = fixture.state.status.clone();

    assert!(!terminal_input_authority::interrupt(
        &mut fixture.state,
        fixture.grant
    ));

    assert_eq!(
        fixture.state.writers[&fixture.target.address].grant,
        replacement
    );
    assert_eq!(fixture.state.status, status);
    assert!(current.validity.as_ref().unwrap().is_current());
    assert!(!terminal_input_authority::blocked(
        &fixture.state,
        fixture.target.address
    ));
    fixture.finish(current, Ok(Reply::Input { duplicate: false }), true);
    assert!(fixture.state.inputs.pending.is_empty());
    fixture.no_jobs();
}

#[test]
fn a_grant_without_a_current_connection_cannot_replace_retained_input_responsibility() {
    let mut fixture = Fixture::new();
    let input = queued(&mut fixture, PREFIX);
    let generation = fixture.state.control_generation.unwrap();
    fixture.state.disconnected();
    let pending = fixture.state.inputs.pending.len();

    terminal_input_stream::granted(&mut fixture.state, fixture.grant, Instant::now());

    assert!(fixture.state.control_generation.is_none());
    assert_eq!(fixture.state.inputs.pending.len(), pending);
    assert_eq!(
        fixture.state.inputs.streams[&fixture.target.address].generation,
        generation
    );
    assert!(fixture.state.writers.is_empty());
    assert!(terminal_input_authority::blocked(
        &fixture.state,
        fixture.target.address
    ));
    fixture.finish(input, unknown(), true);
    assert!(fixture.state.inputs.pending.is_empty());
    fixture.no_jobs();
}
