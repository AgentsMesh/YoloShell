use super::{creation_support::Fixture, finalization_support::*};
use crate::{actions, network::Job};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use yolo_hub_client::ClientError;
use yolo_hub_client_channels::ControlError;
use yolo_hub_rpc_protocol::Command;
use yolo_hub_terminal_protocol::PublicError;

const PREFIX_KEY: char = 'b';
const HELP_KEY: char = '?';
const HUB_MANAGER_KEY: char = 'm';
const PICKER_KEY: char = 'g';

#[derive(Clone, Copy)]
enum PresentationChange {
    ReopenPane,
    Help,
    HubManager,
}

#[test]
fn an_unrelated_status_hint_does_not_hide_a_blocked_finalization() {
    let mut fixture = Fixture::new();
    let (_, _, request) = confirmed(&mut fixture);
    finish(
        &mut fixture,
        request,
        Err(ControlError::Client(ClientError::Remote(
            PublicError::Unauthorized,
        ))),
    );
    assert!(crate::finalization_status::detail(&fixture.state).is_some());
    fixture.state.status = "Selected".into();
    assert!(
        crate::finalization_status::detail(&fixture.state).is_some(),
        "the status line is a display hint, not the connection identity"
    );
}

#[test]
fn delayed_finalize_rejection_does_not_replace_later_pane_or_manager_presentation() {
    for change in [
        PresentationChange::ReopenPane,
        PresentationChange::Help,
        PresentationChange::HubManager,
    ] {
        let mut fixture = Fixture::new();
        let (_, created, request) = confirmed(&mut fixture);
        navigate(&mut fixture, change);
        let tab_catalog = fixture.state.tab_snapshot();
        let selected = fixture.state.selected;
        let mode = fixture.state.mode;
        let status = fixture.state.status.clone();
        let generation = fixture.state.control_generation;
        let help = fixture.state.help;
        let hub_manager = fixture.state.hub_manager.is_some();
        finish(
            &mut fixture,
            request.clone(),
            Err(ControlError::Client(ClientError::Remote(
                PublicError::Unauthorized,
            ))),
        );
        let retained = &fixture.state.creations.finalizing[&token(&request)];
        assert_eq!(retained.record.creation, created);
        assert!(retained.rejection.is_some());
        assert!(retained.attempt.is_none());
        assert_eq!(crate::finalization_status::blocked(&fixture.state), 1);
        assert_eq!(fixture.state.tab_snapshot(), tab_catalog);
        assert_eq!(fixture.state.selected, selected);
        assert_eq!(fixture.state.mode, mode);
        assert_eq!(fixture.state.status, status);
        assert_eq!(fixture.state.control_generation, generation);
        assert_eq!(fixture.state.help, help);
        assert_eq!(fixture.state.hub_manager.is_some(), hub_manager);
        assert!(fixture.state.writers.is_empty());
        crate::finalization_actions::flush(&mut fixture.state, &fixture.jobs, later());
        no_commands(&mut fixture);
    }
}

fn navigate(fixture: &mut Fixture, change: PresentationChange) {
    match change {
        PresentationChange::ReopenPane => {
            press(fixture, KeyCode::Char(PREFIX_KEY), KeyModifiers::CONTROL);
            press(fixture, KeyCode::Char(PICKER_KEY), KeyModifiers::NONE);
            assert_eq!(fixture.state.selected, Some(fixture.expected));
            press(fixture, KeyCode::Enter, KeyModifiers::NONE);
            assert_eq!(fixture.state.focused_terminal(), Some(fixture.expected));
            assert!(matches!(
                fixture.incoming.try_recv().unwrap(),
                Job::Acquire {
                    request,
                    ..
                } if request.target.address == fixture.expected
            ));
        }
        PresentationChange::Help => {
            press(fixture, KeyCode::Char(PREFIX_KEY), KeyModifiers::CONTROL);
            press(fixture, KeyCode::Char(HELP_KEY), KeyModifiers::NONE);
            assert!(fixture.state.help);
        }
        PresentationChange::HubManager => {
            press(fixture, KeyCode::Tab, KeyModifiers::NONE);
            press(fixture, KeyCode::Char(HUB_MANAGER_KEY), KeyModifiers::NONE);
            assert!(fixture.state.hub_manager.is_some());
            let Job::Management(operation) = fixture.incoming.try_recv().unwrap() else {
                panic!("opening Hub management must request its configuration")
            };
            assert_eq!(operation.command, Command::GetConfiguration {});
        }
    }
    assert!(fixture.incoming.try_recv().is_err());
}

fn press(fixture: &mut Fixture, code: KeyCode, modifiers: KeyModifiers) {
    assert!(!actions::handle(
        &mut fixture.state,
        &fixture.jobs,
        fixture.origin,
        Event::Key(KeyEvent::new(code, modifiers)),
    ));
}
