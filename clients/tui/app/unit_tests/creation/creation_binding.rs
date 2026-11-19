use super::{creation_support::*, fixtures};
use crate::Mode;
use yolo_presentation_layout::Axis;

#[test]
fn unchanged_active_creation_binds_and_selects_the_new_resource_then_finalizes_once() {
    let mut fixture = Fixture::new();
    fixture.state.filter = RETAINED_FILTER.into();
    fixture.state.inventory.view = yolo_tui_inventory::View::Agents;
    let request = fixture.start();
    let created = fixture.created();
    fixture.finish(request.clone(), created);
    assert_eq!(fixture.original_binding(), Some(created.address));
    assert_eq!(fixture.state.tab_layout().focused(), fixture.pane);
    assert_eq!(fixture.state.selected, Some(created.address));
    assert_eq!(
        fixture.state.inventory.view,
        yolo_tui_inventory::View::Agents
    );
    assert!(fixture.state.filter.is_empty());
    assert_eq!(fixture.state.mode, Mode::Inventory);
    assert!(fixture.state.creations.pending.is_empty());
    fixture.assert_finalize(&request, created, true);
    fixture.finish(request.clone(), created);
    fixture.assert_finalize(&request, created, false);
}

#[test]
fn delayed_creation_binds_its_original_inactive_tab_without_stealing_current_focus() {
    let mut fixture = Fixture::new();
    let request = fixture.start();
    let active = fixture.switch_tab();
    let view = fixture.state.tab_layout().clone();
    fixture.state.filter = RETAINED_FILTER.into();
    fixture.state.inventory.view = yolo_tui_inventory::View::Hubs;
    let selected = fixture.state.selected;
    let created = fixture.created();
    fixture.finish(request.clone(), created);
    assert_eq!(fixture.original_binding(), Some(created.address));
    assert_eq!(fixture.state.tabs.active().id, active);
    assert_eq!(*fixture.state.tab_layout(), view);
    assert_eq!(fixture.state.selected, selected);
    assert_eq!(fixture.state.filter, RETAINED_FILTER);
    assert_eq!(fixture.state.inventory.view, yolo_tui_inventory::View::Hubs);
    fixture.assert_finalize(&request, created, true);
}

#[test]
fn focus_change_binds_original_pane_without_overwriting_the_now_focused_pane() {
    let mut fixture = Fixture::new();
    let request = fixture.start();
    let other = fixtures::session().fields().address;
    let focused = fixture
        .state
        .tab_layout_mut()
        .split(Axis::Vertical, Some(other))
        .unwrap();
    fixture.state.selected = Some(other);
    let created = fixture.created();
    fixture.finish(request.clone(), created);
    assert_eq!(fixture.original_binding(), Some(created.address));
    assert_eq!(fixture.state.tab_layout().focused(), focused);
    assert_eq!(fixture.state.focused_terminal(), Some(other));
    assert_eq!(fixture.state.selected, Some(other));
    fixture.assert_finalize(&request, created, true);
}

#[test]
fn removed_or_rebound_destinations_keep_created_resource_unassigned_without_cleanup_effects() {
    for destination in [Changed::Rebound, Changed::ClosedPane, Changed::RemovedTab] {
        let mut fixture = Fixture::new();
        let request = fixture.start();
        let replacement = fixtures::session().fields().address;
        match destination {
            Changed::Rebound => fixture
                .state
                .tab_layout_mut()
                .bind(fixture.pane, Some(replacement))
                .unwrap(),
            Changed::ClosedPane => {
                fixture
                    .state
                    .tab_layout_mut()
                    .split(Axis::Horizontal, Some(replacement))
                    .unwrap();
                fixture.state.tab_layout_mut().close(fixture.pane).unwrap();
            }
            Changed::RemovedTab => {
                fixture.switch_tab();
                fixture.state.tabs.remove(fixture.tab).unwrap();
            }
        }
        let layout = fixture.state.tab_layout().clone();
        let books = fixture.state.tab_snapshot();
        let created = fixture.created();
        fixture.finish(request.clone(), created);
        assert_eq!(*fixture.state.tab_layout(), layout);
        assert_eq!(fixture.state.tab_snapshot(), books);
        assert_ne!(fixture.original_binding(), Some(created.address));
        fixture.assert_finalize(&request, created, true);
    }
}

enum Changed {
    Rebound,
    ClosedPane,
    RemovedTab,
}
