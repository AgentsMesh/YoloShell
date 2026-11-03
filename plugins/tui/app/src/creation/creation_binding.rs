use crate::{
    State, creation_constants::CATALOG_BINDING_INVARIANT, creation_state::PendingCreation,
};
use yolo_hub_identity::TerminalAddress;
use yolo_presentation_layout::Layout;
use yolo_tui_tab::{TabCatalog, TabCatalogDocument};

pub(crate) fn apply(
    state: &mut State,
    pending: &PendingCreation,
    created: TerminalAddress,
) -> bool {
    if !pending.binding_active {
        return false;
    }
    if state.tabs.active().id == pending.tab {
        return bind(state.tab_layout_mut(), pending, created);
    }
    let mut document = TabCatalogDocument::from(state.tabs.clone());
    let Some(entry) = document
        .entries
        .iter_mut()
        .find(|entry| entry.id == pending.tab)
    else {
        return false;
    };
    if bind(&mut entry.view.layout, pending, created) {
        state.replace_tabs(TabCatalog::try_from(document).expect(CATALOG_BINDING_INVARIANT));
        return true;
    }
    false
}

fn bind(layout: &mut Layout, pending: &PendingCreation, created: TerminalAddress) -> bool {
    if !layout
        .panes()
        .iter()
        .any(|pane| pane.id == pending.pane && pane.terminal == pending.expected)
    {
        return false;
    }
    layout
        .bind(pending.pane, Some(created))
        .expect(CATALOG_BINDING_INVARIANT);
    true
}
