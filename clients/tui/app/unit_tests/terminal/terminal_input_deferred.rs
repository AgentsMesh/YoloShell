use super::query_support::Fixture;
use crate::{Mode, actions, constants::INPUT_OVERFLOW_LABEL, network::Job, terminal_actions};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use yolo_hub_rpc_protocol::Reply;

const SINGLE_JOB: usize = 1;

#[test]
fn navigation_remains_live_while_terminal_payload_waits_for_transport_capacity() {
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
    assert!(fixture.state.inputs.waiting());

    assert!(!actions::handle(
        &mut fixture.state,
        &fixture.jobs,
        fixture.target.address.hub_id,
        Event::Key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL)),
    ));
    assert!(fixture.state.prefix);
    assert!(fixture.state.inputs.waiting());
    assert!(fixture.state.inputs.deferred.is_empty());
    fixture.state.sync_query_authority();
    assert_eq!(
        fixture.state.writers[&fixture.target.address].grant,
        fixture.grant
    );
    assert!(!actions::handle(
        &mut fixture.state,
        &fixture.jobs,
        fixture.target.address.hub_id,
        Event::Key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE)),
    ));
    fixture.state.sync_query_authority();
    assert_eq!(fixture.state.mode, Mode::Inventory);
    assert!(!fixture.state.inputs.waiting());
    assert_eq!(
        fixture.state.writers[&fixture.target.address].grant,
        fixture.grant
    );
}

#[test]
fn terminal_payload_after_backpressure_keeps_fifo_bytes_and_scope() {
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
    assert_eq!(fixture.state.inputs.deferred[0].grant, fixture.grant);
    assert_eq!(
        fixture.state.inputs.deferred[0].generation,
        fixture.state.control_generation.unwrap()
    );
    assert!(
        fixture.state.inputs.deferred[0]
            .request
            .validity
            .as_ref()
            .unwrap()
            .is_current()
    );
}

#[test]
fn deferred_overflow_stays_fenced_until_explicit_resume() {
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
    for _ in 0..crate::constants::DEFERRED_INPUT_CAPACITY.saturating_sub(1) {
        assert!(terminal_actions::deliver(
            &mut fixture.state,
            &fixture.jobs,
            Ok(b"suffix".to_vec())
        ));
    }
    assert!(!terminal_actions::deliver(
        &mut fixture.state,
        &fixture.jobs,
        Ok(b"overflow".to_vec())
    ));
    assert_eq!(fixture.state.status, INPUT_OVERFLOW_LABEL);
    assert!(
        fixture
            .state
            .inputs
            .overflowed
            .contains(&fixture.target.address)
    );

    assert!(!actions::handle(
        &mut fixture.state,
        &fixture.jobs,
        fixture.target.address.hub_id,
        Event::Key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL)),
    ));
    assert!(!actions::handle(
        &mut fixture.state,
        &fixture.jobs,
        fixture.target.address.hub_id,
        Event::Key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE)),
    ));
    assert!(
        fixture
            .state
            .inputs
            .overflowed
            .contains(&fixture.target.address)
    );

    let prefix = fixture.state.inputs.waiting.take().unwrap();
    fixture.finish(prefix, Ok(Reply::Input { duplicate: false }), true);
    assert!(
        fixture
            .state
            .inputs
            .overflowed
            .contains(&fixture.target.address)
    );
    assert!(fixture.state.inputs.pending.is_empty());
    fixture.state.mode = Mode::Terminal;
    assert!(!terminal_actions::deliver(
        &mut fixture.state,
        &fixture.jobs,
        Ok(b"after-settlement".to_vec())
    ));
    crate::terminal_input_authority::request_resume(&mut fixture.state, fixture.target.address);
    assert!(
        !fixture
            .state
            .inputs
            .overflowed
            .contains(&fixture.target.address)
    );
    assert!(terminal_actions::deliver(
        &mut fixture.state,
        &fixture.jobs,
        Ok(b"after-resume".to_vec())
    ));
    assert!(fixture.state.inputs.waiting());
}
