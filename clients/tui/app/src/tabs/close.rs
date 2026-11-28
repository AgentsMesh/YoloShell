use crate::State;
use anyhow::{Context, Result};
use uuid::Uuid;
use yolo_tui_tab::LocalTabView;

/// Closing presentation retires its pending bindings, never its Terminal resources.
pub(crate) fn tab(state: &mut State, id: Uuid) -> Result<()> {
    state
        .tabs
        .entries()
        .iter()
        .find(|entry| entry.id == id)
        .context(super::LAYOUT_INVARIANT)?;
    super::terminal_picker::close(state);
    let mut catalog = state.tab_snapshot();
    let entry = catalog
        .entries()
        .iter()
        .find(|entry| entry.id == id)
        .context(super::LAYOUT_INVARIANT)?;
    let panes = entry
        .view
        .layout
        .panes()
        .iter()
        .map(|pane| pane.id)
        .collect::<Vec<_>>();
    let active = catalog.active().id == id;
    let last = catalog.count_for(catalog.machine()) == SINGLE_TAB;
    if last {
        catalog.update_active(LocalTabView::default());
    } else {
        catalog.remove(id)?;
    }
    crate::chat::close(state, id, None);
    for pane in panes {
        state.creations.retire_binding(id, pane);
    }
    state.replace_tabs(catalog);
    if active {
        state.activate_tab_collection();
        state.pane_view = Default::default();
        if last {
            super::sidebar(state);
        }
    }
    state.presentation = Default::default();
    super::settle_chat_mode(state);
    state.sync_query_authority();
    Ok(())
}

pub(crate) fn pane(state: &mut State) {
    if state.tab_layout().panes().len() == SINGLE_PANE {
        tab(state, state.tabs.active().id).expect(super::LAYOUT_INVARIANT);
        return;
    }
    super::terminal_picker::close(state);
    let tab = state.tabs.active().id;
    let pane = state.tab_layout().focused();
    crate::chat::close(state, tab, Some(pane));
    state.creations.retire_binding(tab, pane);
    state
        .tab_layout_mut()
        .close(pane)
        .expect(super::LAYOUT_INVARIANT);
    state.presentation = Default::default();
    state.pane_view = Default::default();
    super::settle_chat_mode(state);
    state.sync_query_authority();
}

const SINGLE_TAB: usize = 1;
const SINGLE_PANE: usize = 1;
