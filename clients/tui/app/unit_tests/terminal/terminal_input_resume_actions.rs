use super::query_support::{Fixture, input};
use crate::{
    Mode, actions,
    constants::{CONNECTED_LABEL, DEFERRED_INPUT_CAPACITY, INPUT_OVERFLOW_LABEL},
    network::Job,
    terminal_actions, terminal_input, terminal_input_authority,
};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use yolo_hub_client_channels::{ControlError, ControlRequest};
use yolo_hub_rpc_protocol::Reply;

const SINGLE_JOB: usize = 1;
const FIRST_INPUT: usize = 0;
const PREFIX: &[u8] = b"accepted-prefix";
const ENTER: &[u8] = b"\r";

fn send(fixture: &mut Fixture, data: &[u8]) -> bool {
    terminal_actions::deliver(&mut fixture.state, &fixture.jobs, Ok(data.to_vec()))
}

fn press(fixture: &mut Fixture, code: KeyCode, modifiers: KeyModifiers) {
    assert!(!actions::handle(
        &mut fixture.state,
        &fixture.jobs,
        fixture.target.address.hub_id,
        Event::Key(KeyEvent::new(code, modifiers)),
    ));
    fixture.state.sync_query_authority();
}

fn resume(fixture: &mut Fixture, key: char) {
    press(fixture, KeyCode::Char('b'), KeyModifiers::CONTROL);
    press(fixture, KeyCode::Char(key), KeyModifiers::NONE);
}

fn full() -> Fixture {
    let mut fixture = Fixture::with_capacity(SINGLE_JOB);
    fixture.state.mode = Mode::Terminal;
    fixture
        .jobs
        .try_send(Job::Refresh {
            request_id: uuid::Uuid::new_v4(),
        })
        .unwrap();
    for _ in FIRST_INPUT..DEFERRED_INPUT_CAPACITY {
        assert!(send(&mut fixture, PREFIX));
    }
    assert!(!send(&mut fixture, b"rejected-middle"));
    assert!(
        fixture
            .state
            .inputs
            .overflowed
            .contains(&fixture.target.address)
    );
    fixture
}

fn next(fixture: &mut Fixture) -> ControlRequest {
    terminal_input::flush(&mut fixture.state, fixture.jobs.try_reserve().unwrap());
    let Job::UserInput(request) = fixture.incoming.try_recv().unwrap() else {
        panic!()
    };
    request
}

#[test]
fn actual_acquire_and_takeover_keys_preserve_the_accepted_overflow_prefix() {
    for key in [
        crate::help_constants::ACQUIRE_KEY,
        crate::help_constants::TAKEOVER_KEY,
    ] {
        let mut fixture = full();
        let queued = fixture.state.inputs.waiting.as_ref().unwrap().clone();
        resume(&mut fixture, key);
        assert!(
            !fixture
                .state
                .inputs
                .overflowed
                .contains(&fixture.target.address)
        );
        assert_eq!(fixture.state.inputs.unsent_len(), DEFERRED_INPUT_CAPACITY);
        assert!(queued.validity.as_ref().unwrap().is_current());
        assert_eq!(
            fixture.state.writers[&fixture.target.address].grant,
            fixture.grant
        );
        assert!(matches!(
            fixture.incoming.try_recv(),
            Ok(Job::Refresh { .. })
        ));
        for _ in FIRST_INPUT..DEFERRED_INPUT_CAPACITY {
            let request = next(&mut fixture);
            assert_eq!(input(&request).fields().data, PREFIX);
            assert!(request.validity.as_ref().unwrap().is_current());
            fixture.finish(request, Ok(Reply::Input { duplicate: false }), true);
        }
        assert!(fixture.state.inputs.pending.is_empty());
        assert!(send(&mut fixture, ENTER));
    }
}

#[test]
fn plain_keyboard_resume_still_retires_the_previous_scope() {
    let mut fixture = Fixture::with_capacity(SINGLE_JOB);
    fixture.state.mode = Mode::Terminal;
    assert!(send(&mut fixture, PREFIX));
    let Job::UserInput(request) = fixture.incoming.try_recv().unwrap() else {
        panic!()
    };
    resume(&mut fixture, crate::help_constants::ACQUIRE_KEY);
    assert!(!request.validity.as_ref().unwrap().is_current());
    assert!(fixture.state.inputs.scope.is_none());
}

#[test]
fn overflow_receipts_and_other_messages_do_not_hide_the_lost_input_hint_or_send_enter() {
    let mut fixture = full();
    assert!(matches!(
        fixture.incoming.try_recv(),
        Ok(Job::Refresh { .. })
    ));
    for _ in FIRST_INPUT..DEFERRED_INPUT_CAPACITY {
        let request = next(&mut fixture);
        fixture.finish(request, Ok(Reply::Input { duplicate: false }), true);
    }
    fixture.state.status = CONNECTED_LABEL.into();
    assert_eq!(
        terminal_input_authority::status(&fixture.state),
        Some(INPUT_OVERFLOW_LABEL)
    );
    press(&mut fixture, KeyCode::Enter, KeyModifiers::NONE);
    fixture.no_jobs();
    assert!(fixture.state.inputs.pending.is_empty());
    assert_eq!(
        terminal_input_authority::status(&fixture.state),
        Some(INPUT_OVERFLOW_LABEL)
    );
}

#[test]
fn keyboard_resume_does_not_erase_unknown_outcome_authority() {
    let mut fixture = full();
    assert!(matches!(
        fixture.incoming.try_recv(),
        Ok(Job::Refresh { .. })
    ));
    let request = next(&mut fixture);
    fixture.finish(
        request,
        Err(ControlError::Client(yolo_hub_client::ClientError::Remote(
            yolo_hub_terminal_protocol::PublicError::OutcomeUnknown,
        ))),
        true,
    );
    resume(&mut fixture, crate::help_constants::ACQUIRE_KEY);
    assert!(
        fixture
            .state
            .inputs
            .interrupted
            .contains_key(&fixture.target.address)
    );
    assert!(!fixture.state.writers.contains_key(&fixture.target.address));
    // New bytes wait for the fresh writer confirmed by the explicit resume;
    // they never resume the interrupted stream through an automatic renewal.
    assert!(send(&mut fixture, ENTER));
    assert_eq!(fixture.state.inputs.pregrant.len(), 1);
    while let Ok(job) = fixture.incoming.try_recv() {
        assert!(!matches!(job, Job::UserInput(_)));
    }
    assert_eq!(
        terminal_input_authority::status(&fixture.state),
        Some(terminal_input_authority::RESUMING_HINT),
    );
}
