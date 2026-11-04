use super::{creation_support::Fixture, fixtures};
use crate::{
    Mode,
    network::Job,
    tabs::creation::{self, Placement},
};
use yolo_hub_rpc_protocol::Command;
use yolo_presentation_layout::Axis;

const SINGLE: usize = 1;
const PAIR: usize = 2;
const PAUSED_STORAGE: &str = "creation storage paused";
const INVALID_DIRECTORY: &str = "\0";

fn create(fixture: &mut Fixture, placement: Placement) -> bool {
    creation::create(
        &mut fixture.state,
        &fixture.jobs,
        fixture.origin,
        fixtures::SESSION_DIRECTORY.into(),
        None,
        placement,
    )
}

#[test]
fn new_tab_preserves_existing_terminal_and_late_completion_does_not_change_focus() {
    let mut fixture = Fixture::new();
    let original = fixture.state.tab_layout().clone();
    assert!(create(&mut fixture, Placement::Tab));
    assert_ne!(fixture.state.tabs.active().id, fixture.tab);
    assert_eq!(fixture.state.mode, Mode::Terminal);
    assert_eq!(fixture.original_binding(), Some(fixture.expected));
    assert_eq!(fixture.state.focused_terminal(), None);
    let destination = fixture.state.tabs.active().id;
    let pending = fixture.state.creations.pending.values().next().unwrap();
    assert_eq!(pending.tab, destination);
    assert_eq!(pending.expected, None);
    let request = pending.request.clone();
    let created = fixture.created();
    fixture.state.tabs.select(fixture.tab).unwrap();
    fixture.state.activate_tab_collection();
    crate::creation_events::created(&mut fixture.state, &fixture.jobs, request, created);
    assert_eq!(fixture.state.tabs.active().id, fixture.tab);
    assert_eq!(*fixture.state.tab_layout(), original);
    assert!(fixture.state.initial_control.is_none());
    fixture.state.tabs.select(destination).unwrap();
    fixture.state.activate_tab_collection();
    assert_eq!(fixture.state.focused_terminal(), Some(created.address));
}

#[test]
fn split_reserves_a_new_empty_sibling_and_binds_only_the_new_terminal() {
    for axis in [Axis::Horizontal, Axis::Vertical] {
        let mut fixture = Fixture::new();
        assert!(create(&mut fixture, Placement::Split(axis)));
        assert_eq!(fixture.state.tabs.active().id, fixture.tab);
        assert_eq!(fixture.state.tab_layout().panes().len(), PAIR);
        assert_ne!(fixture.state.tab_layout().focused(), fixture.pane);
        let pending = fixture.state.creations.pending.values().next().unwrap();
        assert_eq!(pending.expected, None);
        assert_eq!(pending.pane, fixture.state.tab_layout().focused());
        let request = pending.request.clone();
        let created = fixture.created();
        crate::creation_events::created(&mut fixture.state, &fixture.jobs, request, created);
        assert_eq!(fixture.state.focused_terminal(), Some(created.address));
        assert_eq!(fixture.original_binding(), Some(fixture.expected));
        assert!(fixture.state.initial_control.is_some());
    }
}

#[test]
fn initial_empty_tab_is_reused_but_pending_creation_reserves_its_destination() {
    let mut fixture = Fixture::new();
    fixture
        .state
        .tab_layout_mut()
        .bind(fixture.pane, None)
        .unwrap();
    assert!(create(&mut fixture, Placement::Empty));
    assert_eq!(fixture.state.tabs.entries().len(), SINGLE);
    let first = fixture
        .state
        .creations
        .pending
        .values()
        .next()
        .unwrap()
        .request
        .clone();
    assert!(create(&mut fixture, Placement::Tab));
    assert_eq!(fixture.state.tabs.entries().len(), PAIR);
    assert_eq!(fixture.state.creations.pending.len(), PAIR);
    let pending = &fixture.state.creations.pending[&first.fields().request_id];
    assert!(pending.binding_active);
    assert_eq!(pending.tab, fixture.tab);
    assert!(matches!(
        fixture.incoming.try_recv().unwrap(),
        Job::Call {
            command: Command::Create { .. },
            ..
        }
    ));
}

#[test]
fn empty_placement_cannot_replace_an_existing_terminal() {
    let mut fixture = Fixture::new();
    assert!(create(&mut fixture, Placement::Empty));
    assert_eq!(fixture.original_binding(), Some(fixture.expected));
    assert_ne!(fixture.state.tabs.active().id, fixture.tab);
}

#[test]
fn failed_admission_restores_layout_focus_mode_selection_and_zoom() {
    for placement in [Placement::Tab, Placement::Split(Axis::Horizontal)] {
        for failure in [
            Failure::Queue,
            Failure::Disconnected,
            Failure::Storage,
            Failure::Invalid,
        ] {
            let mut fixture = Fixture::with_capacity(SINGLE);
            fixture.state.mode = Mode::Inventory;
            fixture.state.pane_view.zoomed = true;
            fixture.state.prefix = true;
            let before = fixture.state.tab_snapshot();
            let selected = fixture.state.selected;
            match failure {
                Failure::Queue => fixture
                    .jobs
                    .try_send(Job::Refresh {
                        request_id: uuid::Uuid::new_v4(),
                    })
                    .unwrap(),
                Failure::Disconnected => fixture.state.control_generation = None,
                Failure::Storage => {
                    fixture.state.creations.persistence_error = Some(PAUSED_STORAGE)
                }
                Failure::Invalid => {}
            }
            let cwd = if matches!(failure, Failure::Invalid) {
                INVALID_DIRECTORY
            } else {
                fixtures::SESSION_DIRECTORY
            };
            assert!(!creation::create(
                &mut fixture.state,
                &fixture.jobs,
                fixture.origin,
                cwd.into(),
                None,
                placement
            ));
            assert_eq!(fixture.state.tab_snapshot(), before);
            assert_eq!(fixture.state.selected, selected);
            assert_eq!(fixture.state.mode, Mode::Inventory);
            assert!(fixture.state.pane_view.zoomed);
            assert!(fixture.state.prefix);
            assert!(fixture.state.creations.pending.is_empty());
        }
    }
}

enum Failure {
    Queue,
    Disconnected,
    Storage,
    Invalid,
}

#[test]
fn exhausted_tab_capacity_never_drops_the_current_terminal() {
    let mut fixture = Fixture::new();
    for _ in SINGLE..yolo_tui_tab::MAX_TABS {
        assert!(create(&mut fixture, Placement::Tab));
        fixture.incoming.try_recv().unwrap();
    }
    let before = fixture.state.tab_snapshot();
    assert!(!create(&mut fixture, Placement::Tab));
    assert_eq!(fixture.state.tab_snapshot(), before);
    assert_eq!(fixture.original_binding(), Some(fixture.expected));
}
