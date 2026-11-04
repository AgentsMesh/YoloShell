use super::{creation_support, fixtures, query_support::Fixture};
use crate::{Mode, actions, network::Job};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use yolo_hub_rpc_protocol::Command;

fn key(fixture: &mut Fixture, code: KeyCode, modifiers: KeyModifiers) {
    actions::handle(
        &mut fixture.state,
        &fixture.jobs,
        fixture.target.address.hub_id,
        Event::Key(KeyEvent::new(code, modifiers)),
    );
}

#[test]
fn repeated_control_b_reaches_the_shell_but_plain_b_toggles_the_sidebar() {
    let mut fixture = Fixture::new();
    fixture.state.mode = Mode::Terminal;
    for _ in [(), ()] {
        key(&mut fixture, KeyCode::Char('b'), KeyModifiers::CONTROL);
    }
    let Job::UserInput(request) = fixture.incoming.try_recv().unwrap() else {
        panic!()
    };
    let Command::Input { request } = request.command else {
        panic!()
    };
    assert_eq!(request.fields().data, vec![crate::tabs::LITERAL_PREFIX]);
    assert!(!fixture.state.sidebar_visible);
    key(&mut fixture, KeyCode::Char('b'), KeyModifiers::CONTROL);
    key(&mut fixture, KeyCode::Char('b'), KeyModifiers::NONE);
    assert!(fixture.state.sidebar_visible);
    fixture.no_jobs();
}

#[test]
fn typing_while_new_tab_starts_does_not_cancel_initial_control_or_replay_input() {
    let mut fixture = creation_support::Fixture::new();
    assert!(crate::shell_actions::create(
        &mut fixture.state,
        &fixture.jobs,
        fixture.origin,
        fixtures::SESSION_DIRECTORY.into(),
        None
    ));
    let Job::Call {
        command: Command::Create { request },
        ..
    } = fixture.incoming.try_recv().unwrap()
    else {
        panic!()
    };
    let epoch = fixture.state.ui_epoch;
    for event in [
        Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        Event::Paste("early input".into()),
    ] {
        actions::handle(&mut fixture.state, &fixture.jobs, fixture.origin, event);
    }
    assert_eq!(fixture.state.ui_epoch, epoch);
    assert!(fixture.incoming.try_recv().is_err());
    let created = fixture.created();
    crate::creation_events::created(&mut fixture.state, &fixture.jobs, request, created);
    assert!(fixture.state.initial_control.is_some());
    assert_eq!(fixture.state.focused_terminal(), Some(created.address));
    assert_eq!(fixture.original_binding(), Some(fixture.expected));
}

#[test]
fn tab_selection_from_picker_closes_overlay_before_input_can_be_delivered() {
    let mut fixture = Fixture::new();
    fixture.state.mode = Mode::Terminal;
    crate::tabs::terminal_picker::open(&mut fixture.state);
    assert!(fixture.state.terminal_picker.is_some());
    let tab = fixture.state.tabs.active().id;
    assert!(crate::tabs::activate(&mut fixture.state, tab));
    assert!(fixture.state.terminal_picker.is_none());
    assert_eq!(fixture.state.mode, Mode::Terminal);
    assert_eq!(
        fixture.state.focused_terminal(),
        Some(fixture.target.address)
    );
}
