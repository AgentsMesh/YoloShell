use super::{creation_support::Fixture, fixtures};
use crate::{Mode, network::Job, tabs};
use crossterm::event::{Event, KeyCode};
use yolo_hub_rpc_protocol::Command;
use yolo_tui_tab_manager::Manager;

const OTHER_TAB: &str = "Other";
const SINGLE_TAB: usize = 1;

#[test]
fn both_tab_close_routes_leave_the_same_empty_last_view_and_preserve_terminals() {
    for managed in [false, true] {
        let mut fixture = Fixture::new();
        let tab = fixture.state.tabs.active().id;
        let name = fixture.state.tabs.active().name.clone();
        if managed {
            fixture.state.tab_manager = Some(Manager::new(&fixture.state.tabs));
            for key in [KeyCode::Char('x'), KeyCode::Enter] {
                tabs::manager::handle(
                    &mut fixture.state,
                    &fixture.jobs,
                    fixture.origin,
                    Event::Key(key.into()),
                );
            }
        } else {
            tabs::prefix(
                &mut fixture.state,
                &fixture.jobs,
                fixture.origin,
                KeyCode::Char(tabs::CLOSE_TAB_KEY),
            );
        }
        assert_eq!(fixture.state.tabs.entries().len(), SINGLE_TAB);
        assert_eq!(fixture.state.tabs.active().id, tab);
        assert_eq!(fixture.state.tabs.active().name, name);
        assert!(fixture.state.focused_terminal().is_none());
        assert_eq!(fixture.state.mode, Mode::Inventory);
        assert!(fixture.state.terminals.contains_key(&fixture.expected));
        assert!(fixture.incoming.try_recv().is_err());
    }
}

#[test]
fn closing_inactive_pending_tab_preserves_active_focus_and_retires_late_binding() {
    let mut fixture = Fixture::new();
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
    let pending = fixture.state.tabs.active().id;
    fixture.state.tabs.create(OTHER_TAB).unwrap();
    fixture.state.activate_tab_collection();
    let current = fixture.state.tabs.active().id;
    let layout = fixture.state.tab_layout().clone();
    tabs::close::tab(&mut fixture.state, pending).unwrap();
    assert_eq!(fixture.state.tabs.active().id, current);
    assert_eq!(*fixture.state.tab_layout(), layout);
    assert!(!fixture.state.creations.pending[&request.fields().request_id].binding_active);
    let created = fixture.created();
    crate::creation_events::created(&mut fixture.state, &fixture.jobs, request, created);
    assert_eq!(fixture.state.tabs.active().id, current);
    assert_eq!(*fixture.state.tab_layout(), layout);
    assert!(!fixture.state.bound_terminals().contains(&created.address));
    while let Ok(job) = fixture.incoming.try_recv() {
        assert!(matches!(job, Job::Finalize { .. } | Job::Refresh { .. }));
    }
}
