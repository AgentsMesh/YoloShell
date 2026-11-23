use crate::{Mode, State, network::Job};
use tokio::sync::mpsc;
use uuid::Uuid;
use yolo_hub_identity::TerminalAddress;
use yolo_tui_tab::{LocalTabView, TabViewMode};

pub(crate) fn activate(state: &mut State, id: Uuid) -> bool {
    let mut catalog = state.tab_snapshot();
    if let Err(error) = catalog.select(id) {
        state.status = error.to_string();
        return false;
    }
    super::terminal_picker::close(state);
    state.replace_tabs(catalog);
    state.activate_tab_collection();
    state.mode = Mode::Terminal;
    true
}

pub(crate) fn open(state: &mut State, jobs: &mpsc::Sender<Job>, address: TerminalAddress) {
    if crate::terminal_lifecycle::gone(state, address) {
        crate::startup::retained(state, Some(address));
        return;
    }
    state.inventory.remember_terminal(address);
    if state.focused_terminal() == Some(address) {
        super::terminal_picker::close(state);
        state.mode = Mode::Terminal;
        state.selected = Some(address);
        crate::polling::acquire(state, jobs, false);
        return;
    }
    let mut catalog = state.tab_snapshot();
    let existing = catalog.entries().iter().find_map(|entry| {
        entry
            .view
            .layout
            .panes()
            .into_iter()
            .find(|pane| pane.terminal == Some(address))
            .map(|pane| (entry.id, pane.id))
    });
    if let Some((tab, pane)) = existing {
        catalog.select(tab).expect(super::LAYOUT_INVARIANT);
        super::terminal_picker::close(state);
        state.replace_tabs(catalog);
        state.activate_tab_collection();
        state
            .tab_layout_mut()
            .focus(pane)
            .expect(super::LAYOUT_INVARIANT);
    } else {
        let empty = state.tab_layout().panes().len() == 1
            && state.focused_terminal().is_none()
            && !state
                .creations
                .pending
                .values()
                .any(|pending| pending.tab == catalog.active().id);
        if !empty {
            let name = match super::creation::tab_name(&catalog) {
                Ok(name) => name,
                Err(error) => {
                    state.status = error.to_string();
                    return;
                }
            };
            catalog.create(&name).expect(super::LAYOUT_INVARIANT);
        }
        let mut layout = catalog.active().view.layout.clone();
        layout
            .bind(layout.focused(), Some(address))
            .expect(super::LAYOUT_INVARIANT);
        catalog.update_active(LocalTabView {
            layout,
            view: TabViewMode::Terminal,
        });
        super::terminal_picker::close(state);
        state.replace_tabs(catalog);
        state.activate_tab_collection();
    }
    state
        .creations
        .retire_binding(state.tabs.active().id, state.tab_layout().focused());
    state.selected = Some(address);
    state.mode = Mode::Terminal;
    crate::polling::acquire(state, jobs, false);
}
