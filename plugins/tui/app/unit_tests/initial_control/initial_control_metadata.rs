use super::initial_control_late_grants::{finish, pending_acquire};
use crate::{Mode, actions, network::Job};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

const COLUMNS: u16 = 100;
const ROWS: u16 = 30;
const TYPED_CHARACTER: char = 'q';
const PASTE: &str = "claude\r";

#[test]
fn resize_focus_and_key_release_do_not_revoke_a_delayed_initial_grant() {
    for event in [
        Event::Resize(COLUMNS, ROWS),
        Event::FocusGained,
        Event::FocusLost,
        Event::Key(KeyEvent::new_with_kind(
            KeyCode::Char(TYPED_CHARACTER),
            KeyModifiers::NONE,
            KeyEventKind::Release,
        )),
    ] {
        let (mut fixture, request, grant) = pending_acquire();
        let epoch = fixture.state.ui_epoch;
        assert!(!actions::handle(
            &mut fixture.state,
            &fixture.jobs,
            fixture.origin,
            event
        ));
        assert_eq!(fixture.state.ui_epoch, epoch);
        assert!(request.validity.as_ref().unwrap().is_current());
        finish(&mut fixture, request, grant);
        assert_eq!(
            fixture.state.writers[&grant.fields().target.address].grant,
            grant
        );
        assert_eq!(fixture.state.mode, Mode::Terminal);
        assert!(fixture.incoming.try_recv().is_err());
    }
}

#[test]
fn early_terminal_typing_is_buffered_for_exact_acquisition_and_replayed_after_grant() {
    for event in [
        Event::Key(KeyCode::Char(TYPED_CHARACTER).into()),
        Event::Paste(PASTE.into()),
    ] {
        let expected = match &event {
            Event::Key(_) => TYPED_CHARACTER.to_string().into_bytes(),
            Event::Paste(_) => PASTE.as_bytes().to_vec(),
            _ => unreachable!(),
        };
        let (mut fixture, request, grant) = pending_acquire();
        assert!(crate::input_intent::waiting_in_terminal(&fixture.state));
        let address = grant.fields().target.address;
        let epoch = fixture.state.ui_epoch;
        assert!(!actions::handle(
            &mut fixture.state,
            &fixture.jobs,
            fixture.origin,
            event
        ));
        assert_eq!(fixture.state.ui_epoch, epoch);
        assert!(request.validity.as_ref().unwrap().is_current());
        assert_eq!(fixture.state.status, crate::constants::INPUT_WAITING_LABEL);
        assert!(fixture.incoming.try_recv().is_err());
        assert_eq!(fixture.state.inputs.pregrant.len(), 1);
        finish(&mut fixture, request, grant);
        assert_eq!(fixture.state.focused_terminal(), Some(address));
        assert!(fixture.state.writers.contains_key(&address));
        assert_eq!(fixture.state.status, crate::constants::CONNECTED_LABEL);
        assert!(fixture.state.inputs.pregrant.is_empty());
        let Job::UserInput(input) = fixture.incoming.try_recv().unwrap() else {
            panic!("initial input must be sent after the grant");
        };
        let yolo_hub_rpc_protocol::Command::Input { request } = input.command else {
            panic!("initial input must use the input command");
        };
        assert_eq!(request.fields().data, expected);
        assert!(fixture.incoming.try_recv().is_err());
    }
}

#[test]
fn multiple_pregrant_payloads_keep_terminal_order_after_the_grant() {
    let (mut fixture, request, grant) = pending_acquire();
    for event in [
        Event::Key(KeyCode::Char('a').into()),
        Event::Key(KeyCode::Char('b').into()),
        Event::Paste("c".into()),
    ] {
        assert!(!actions::handle(
            &mut fixture.state,
            &fixture.jobs,
            fixture.origin,
            event,
        ));
    }
    // Adjacent bytes coalesce into one batch; the terminal order is unchanged.
    assert_eq!(fixture.state.inputs.pregrant.len(), 1);
    finish(&mut fixture, request, grant);

    let Job::UserInput(input) = fixture.incoming.try_recv().unwrap() else {
        panic!("pregrant input must use the input queue");
    };
    let yolo_hub_rpc_protocol::Command::Input { request } = input.command else {
        panic!("pregrant input must use the input command");
    };
    assert_eq!(request.fields().data, b"abc");
    assert!(fixture.incoming.try_recv().is_err());
}

#[test]
fn navigating_away_retires_pregrant_bytes_with_their_initial_control_intent() {
    let (mut fixture, request, grant) = pending_acquire();
    assert!(
        actions::handle(
            &mut fixture.state,
            &fixture.jobs,
            fixture.origin,
            Event::Key(KeyCode::Char(TYPED_CHARACTER).into()),
        ) == false
    );
    assert_eq!(fixture.state.inputs.pregrant.len(), 1);

    fixture.state.prefix = true;
    actions::handle(
        &mut fixture.state,
        &fixture.jobs,
        fixture.origin,
        Event::Key(KeyCode::Char(crate::help_constants::SESSIONS_KEY).into()),
    );
    assert!(fixture.state.inputs.pregrant.is_empty());
    finish(&mut fixture, request, grant);
    assert!(fixture.incoming.try_recv().is_ok());
}

#[test]
fn pending_new_shell_presentation_is_still_cancelled_by_typing_into_the_old_shell() {
    let mut fixture = super::creation_support::Fixture::new();
    fixture.state.mode = Mode::Terminal;
    let epoch = fixture.state.ui_epoch;
    actions::handle(
        &mut fixture.state,
        &fixture.jobs,
        fixture.origin,
        Event::Key(KeyCode::Char(TYPED_CHARACTER).into()),
    );
    assert!(fixture.state.ui_epoch > epoch);
    assert!(fixture.state.initial_control.is_none());
}
