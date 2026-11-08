use super::{panes, selection_support::*};
use crate::{Mode, actions, selection_actions};
use crossterm::event::Event;
use yolo_hub_terminal_protocol::{
    TerminalSize, TerminalSnapshot, TerminalSnapshotFields, TerminalSummary,
};
use yolo_tui_terminal::TerminalSurface;

#[test]
fn focus_loss_and_resize_cancel_the_drag_without_input_or_copy() {
    for event in [
        Event::FocusLost,
        Event::Resize(panes::WIDTH + NEXT, panes::HEIGHT),
    ] {
        let mut fixture = Fixture::new();
        fixture.begin(FIRST, FIRST);
        actions::handle(
            &mut fixture.state,
            &fixture.jobs,
            fixture.target.address.hub_id,
            event,
        );
        fixture.finish(LAST_ALPHA, FIRST);
        fixture.copy();
        assert!(selection_actions::take_clipboard(&mut fixture.state).is_none());
        assert!(fixture.state.selection.active.is_none());
        fixture.no_input();
    }
}

#[derive(Clone, Copy)]
enum Change {
    Focus,
    Layout,
    Runtime,
    Size,
    MissingSurface,
    Viewport,
    Zoom,
    Mode,
}

#[test]
fn source_and_layout_changes_fence_frozen_selection_before_copy() {
    for change in [
        Change::Focus,
        Change::Layout,
        Change::Runtime,
        Change::Size,
        Change::MissingSurface,
        Change::Viewport,
        Change::Zoom,
        Change::Mode,
    ] {
        let mut fixture = Fixture::new();
        fixture.select();
        match change {
            Change::Focus => {
                let other = fixture
                    .state
                    .tab_layout()
                    .panes()
                    .into_iter()
                    .find(|pane| pane.id != fixture.view.pane)
                    .map(|pane| pane.id)
                    .unwrap();
                fixture.state.tab_layout_mut().focus(other).unwrap();
            }
            Change::Layout => {
                fixture
                    .state
                    .tab_layout_mut()
                    .split(yolo_presentation_layout::Axis::Vertical, None)
                    .unwrap();
            }
            Change::Runtime => {
                let mut fields = fixture.state.terminals[&fixture.target.address]
                    .fields()
                    .clone();
                fields.runtime.as_mut().unwrap().target.runtime_epoch =
                    yolo_hub_identity::RuntimeEpoch::new();
                fixture.state.terminals.insert(
                    fixture.target.address,
                    TerminalSummary::new(fields).unwrap(),
                );
            }
            Change::Size => {
                let snapshot = TerminalSnapshot::new(TerminalSnapshotFields {
                    target: fixture.target,
                    barrier_sequence: 0,
                    size: TerminalSize::new(
                        fixture.view.inner.width + NEXT,
                        fixture.view.inner.height,
                    )
                    .unwrap(),
                    data: TEXT.to_vec(),
                })
                .unwrap();
                fixture
                    .state
                    .surfaces
                    .insert(fixture.target.address, TerminalSurface::new(&snapshot));
            }
            Change::MissingSurface => {
                fixture.state.surfaces.remove(&fixture.target.address);
            }
            Change::Viewport => {
                fixture
                    .state
                    .presentation
                    .panes
                    .iter_mut()
                    .find(|view| view.pane == fixture.view.pane)
                    .unwrap()
                    .inner
                    .x += NEXT;
            }
            Change::Zoom => fixture.state.pane_view.zoomed = !fixture.state.pane_view.zoomed,
            Change::Mode => fixture.state.mode = Mode::Inventory,
        }
        assert!(
            selection_actions::display_screen(&fixture.state, fixture.target.address).is_none()
        );
        assert!(selection_actions::highlight(&fixture.state, fixture.view).is_none());
        selection_actions::copy(&mut fixture.state);
        assert!(fixture.state.selection.active.is_none());
        assert!(selection_actions::take_clipboard(&mut fixture.state).is_none());
        fixture.no_input();
    }
}
