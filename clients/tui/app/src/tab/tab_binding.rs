use crate::State;
use std::collections::BTreeSet;
use yolo_hub_identity::TerminalAddress;
use yolo_tui_tab::{LocalTabView, TabCatalog};

/// What a tab-catalog admission changed. Callers that own focus policy use the
/// active flag; catalog bulk setters can ignore it.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct Admission {
    pub retired: bool,
    pub active_affected: bool,
}

/// Terminal addresses that can no longer own a live presentation.
pub(crate) fn gone_addresses(state: &State) -> BTreeSet<TerminalAddress> {
    state
        .terminals
        .keys()
        .copied()
        .filter(|address| crate::terminal_lifecycle::gone(state, *address))
        .collect()
}

/// The one admission point for every tab catalog write.
///
/// Local edits, restore, shared-layout adoption and catalogue refreshes all
/// pass through here. A pane that binds a gone terminal is removed before the
/// catalog is installed, so a remote document cannot reintroduce a dead pane.
pub(crate) fn admit(state: &mut State, mut catalog: TabCatalog) -> (TabCatalog, Admission) {
    let gone = gone_addresses(state);
    if gone.is_empty() {
        return (catalog, Admission::default());
    }
    let active = catalog.active().id;
    let mut admission = Admission::default();
    for entry in catalog.entries().to_vec() {
        let panes = entry
            .view
            .layout
            .panes()
            .into_iter()
            .map(|pane| (pane.id, pane.terminal))
            .collect::<Vec<_>>();
        let dead = panes
            .iter()
            .filter_map(|(pane, terminal)| {
                terminal
                    .filter(|address| gone.contains(address))
                    .map(|_| *pane)
            })
            .collect::<Vec<_>>();
        if dead.is_empty() {
            continue;
        }
        admission.retired = true;
        for pane in &dead {
            state.creations.retire_binding(entry.id, *pane);
            crate::chat::close(state, entry.id, Some(*pane));
        }
        let empty_tab = panes
            .iter()
            .all(|(_, terminal)| terminal.is_some_and(|address| gone.contains(&address)));
        if empty_tab {
            crate::chat::close(state, entry.id, None);
            if catalog.entries().len() == 1 {
                catalog.update_active(LocalTabView::default());
            } else {
                catalog
                    .remove(entry.id)
                    .expect(crate::tabs::LAYOUT_INVARIANT);
            }
        } else {
            let mut layout = entry.view.layout;
            for pane in dead {
                layout.close(pane).expect(crate::tabs::LAYOUT_INVARIANT);
            }
            catalog
                .replace_view(
                    entry.id,
                    LocalTabView {
                        layout,
                        view: entry.view.view,
                    },
                )
                .expect(crate::tabs::LAYOUT_INVARIANT);
        }
        admission.active_affected |= entry.id == active;
    }
    (catalog, admission)
}
