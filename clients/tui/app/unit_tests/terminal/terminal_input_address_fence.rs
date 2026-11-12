use super::{fixtures, query_support::Fixture};
use crate::{Mode, network::Job, terminal_actions};
use std::time::Instant;
use tokio::sync::mpsc;
use yolo_hub_terminal_protocol::{InputFields, InputRequest};

const SINGLE_JOB: usize = 1;

#[test]
fn one_terminal_overflow_does_not_fence_another_terminal() {
    let mut fixture = Fixture::with_capacity(SINGLE_JOB);
    let current_session = fixture.state.terminals[&fixture.target.address].clone();
    let other_session = fixtures::session();
    let other_grant = fixtures::grant(&other_session);
    let other_address = other_grant.fields().target.address;
    fixture
        .state
        .replace_terminals(vec![current_session, other_session]);
    fixture.state.record_grant(other_grant, Instant::now());
    fixture
        .state
        .inputs
        .overflowed
        .insert(fixture.target.address);
    let input = InputRequest::new(InputFields {
        request_id: uuid::Uuid::new_v4(),
        grant: other_grant,
        data: b"other-terminal".to_vec(),
    })
    .unwrap();
    let (jobs, mut received) = mpsc::channel(SINGLE_JOB);
    assert!(crate::terminal_input::enqueue(
        &mut fixture.state,
        &jobs,
        input
    ));
    assert!(matches!(received.try_recv(), Ok(Job::UserInput(_))));
    assert!(!fixture.state.inputs.overflowed.contains(&other_address));
}

#[test]
fn writer_replacement_retires_deferred_suffix_without_reinterpreting_it() {
    let mut fixture = Fixture::with_capacity(SINGLE_JOB);
    fixture.state.mode = Mode::Terminal;
    fixture
        .jobs
        .try_send(Job::Refresh {
            request_id: uuid::Uuid::new_v4(),
        })
        .unwrap();
    assert!(terminal_actions::deliver(
        &mut fixture.state,
        &fixture.jobs,
        Ok(b"prefix".to_vec())
    ));
    assert!(terminal_actions::deliver(
        &mut fixture.state,
        &fixture.jobs,
        Ok(b"suffix".to_vec())
    ));
    assert_eq!(fixture.state.inputs.deferred.len(), 1);
    crate::terminal_input_authority::request_resume(&mut fixture.state, fixture.target.address);
    let replacement = fixture.replace_writer();
    fixture.state.sync_query_authority();
    assert!(fixture.state.inputs.deferred.is_empty());
    assert!(!fixture.state.inputs.waiting());
    assert!(
        !fixture
            .state
            .inputs
            .overflowed
            .contains(&fixture.target.address)
    );
    assert_eq!(
        fixture.state.writers[&fixture.target.address].grant,
        replacement
    );
}
