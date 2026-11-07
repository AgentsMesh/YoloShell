use super::creation_support::Fixture;
use crate::{
    Mode, events,
    network::{Job, Update},
};
use yolo_hub_client::ClientError;
use yolo_hub_client_channels::{ControlError, ControlEvent, ControlRequest};
use yolo_hub_rpc_protocol::Command;
use yolo_hub_terminal_protocol::PublicError;
use yolo_tui_inventory::View;

const DIRECTORY: &str = "/missing/directory with spaces";
const RENAMED: &str = "User changed this tab";
const NEXT_TAB: &str = "User's next tab";

fn only_refresh(fixture: &mut Fixture) {
    while let Ok(job) = fixture.incoming.try_recv() {
        assert!(matches!(job, Job::Refresh { .. }));
    }
}

fn directory_request(fixture: &mut Fixture) -> ControlRequest {
    crate::tabs::terminal_picker::open(&mut fixture.state);
    fixture.state.mode = Mode::Directory;
    fixture.state.entry = DIRECTORY.into();
    crate::form_input::reset(&mut fixture.state);
    assert!(crate::shell_actions::create(
        &mut fixture.state,
        &fixture.jobs,
        fixture.origin,
        DIRECTORY.into(),
        None
    ));
    let Job::Call {
        generation,
        origin,
        command,
    } = fixture.incoming.try_recv().unwrap()
    else {
        panic!()
    };
    ControlRequest {
        generation,
        destination: origin,
        command,
        validity: None,
    }
}

fn rejected(fixture: &mut Fixture, request: ControlRequest, unknown: bool) {
    let error = if unknown {
        ClientError::OutcomeUnknownResponse(PublicError::InvalidDirectory)
    } else {
        ClientError::Remote(PublicError::InvalidDirectory)
    };
    events::apply(
        &mut fixture.state,
        &fixture.jobs,
        Update::Control(ControlEvent::Finished {
            request,
            result: Err(ControlError::Client(error)),
            connected: true,
        }),
    );
}

#[test]
fn rejected_directory_restores_the_original_form_and_layout_without_an_empty_tab() {
    let mut fixture = Fixture::new();
    let before = fixture.state.tab_snapshot();
    let request = directory_request(&mut fixture);
    assert!(fixture.state.tabs.entries().len() > before.entries().len());
    rejected(&mut fixture, request, false);
    assert_eq!(fixture.state.tab_snapshot(), before);
    assert_eq!(fixture.state.mode, Mode::Directory);
    assert_eq!(fixture.state.entry, DIRECTORY);
    assert_eq!(fixture.state.inventory.view, View::Terminals);
    assert!(fixture.state.terminal_picker.is_some());
    assert_eq!(fixture.state.focused_terminal(), Some(fixture.expected));
    assert!(fixture.state.creations.pending.is_empty());
    assert!(fixture.state.creations.recovery.is_empty());
    assert!(fixture.state.creations.uncreated.is_empty());
    assert!(fixture.state.status.contains("directory"));
    only_refresh(&mut fixture);
}

#[test]
fn late_rejection_never_overwrites_a_renamed_rebound_or_new_active_view() {
    for change in [Change::Rename, Change::Bind, Change::Navigate] {
        let mut fixture = Fixture::new();
        let request = directory_request(&mut fixture);
        match change {
            Change::Rename => fixture
                .state
                .tabs
                .rename(fixture.state.tabs.active().id, RENAMED)
                .unwrap(),
            Change::Bind => fixture.state.bind_focused(Some(fixture.expected)).unwrap(),
            Change::Navigate => {
                fixture.state.tabs.create(NEXT_TAB).unwrap();
                fixture.state.activate_tab_collection();
            }
        }
        let before = fixture.state.tab_snapshot();
        let mode = fixture.state.mode;
        rejected(&mut fixture, request, false);
        assert_eq!(fixture.state.tab_snapshot(), before);
        assert_eq!(fixture.state.mode, mode);
        assert!(fixture.state.creations.pending.is_empty());
        assert!(fixture.state.creations.recovery.is_empty());
        only_refresh(&mut fixture);
    }
}

#[test]
fn unknown_delivery_preserves_the_reservation_and_durable_recovery_instead_of_undoing_it() {
    let mut fixture = Fixture::new();
    let request = directory_request(&mut fixture);
    let before = fixture.state.tab_snapshot();
    rejected(&mut fixture, request, true);
    assert_eq!(fixture.state.tab_snapshot(), before);
    assert_eq!(fixture.state.mode, Mode::Terminal);
    assert!(!fixture.state.creations.recovery.is_empty());
    assert!(fixture.state.creations.uncreated.is_empty());
    only_refresh(&mut fixture);
}

#[test]
fn proven_unsent_split_undoes_only_its_provisional_pane_and_keeps_the_old_terminal() {
    let mut fixture = Fixture::new();
    fixture.state.mode = Mode::Terminal;
    let before = fixture.state.tab_snapshot();
    crate::tabs::prefix(
        &mut fixture.state,
        &fixture.jobs,
        fixture.origin,
        crossterm::event::KeyCode::Char(crate::tabs::SPLIT_RIGHT_KEY),
    );
    let Job::Call {
        generation,
        origin,
        command,
    } = fixture.incoming.try_recv().unwrap()
    else {
        panic!()
    };
    events::apply(
        &mut fixture.state,
        &fixture.jobs,
        Update::NotSent {
            request: ControlRequest {
                generation,
                destination: origin,
                command,
                validity: None,
            },
        },
    );
    assert_eq!(fixture.state.tab_snapshot(), before);
    assert_eq!(fixture.state.focused_terminal(), Some(fixture.expected));
    assert!(fixture.incoming.try_recv().is_err());
}

#[test]
fn stale_rejection_after_success_cannot_remove_a_created_binding() {
    let mut fixture = Fixture::new();
    let request = directory_request(&mut fixture);
    let Command::Create {
        request: creation_request,
    } = &request.command
    else {
        panic!()
    };
    let created = fixture.created();
    crate::creation_events::created(
        &mut fixture.state,
        &fixture.jobs,
        creation_request.clone(),
        created,
    );
    let before = fixture.state.tab_snapshot();
    rejected(&mut fixture, request, false);
    assert_eq!(fixture.state.tab_snapshot(), before);
    assert!(fixture.state.bound_terminals().contains(&created.address));
}

enum Change {
    Rename,
    Bind,
    Navigate,
}
