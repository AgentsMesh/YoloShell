use crate::{Mode, State, creation_state::PendingCreation};
use yolo_hub_identity::HubId;
use yolo_tui_inventory::View;

#[derive(Clone)]
pub(crate) struct CreationPresentation {
    filter: String,
    scope: Option<HubId>,
    view: View,
    mode: Mode,
}

impl CreationPresentation {
    pub fn capture(state: &State) -> Self {
        Self {
            filter: state.filter.clone(),
            scope: state.inventory.scope,
            view: state.inventory.view,
            mode: if state.mode == Mode::Directory {
                Mode::Inventory
            } else {
                state.mode
            },
        }
    }

    pub fn matches(&self, state: &State) -> bool {
        state.filter == self.filter
            && state.inventory.scope == self.scope
            && state.inventory.view == self.view
            && state.mode == self.mode
            && state.tab_manager.is_none()
            && state.hub_manager.is_none()
            && !state.help
    }

    pub fn restore_directory(self, state: &mut State) {
        state.filter = self.filter;
        state.inventory.scope = self.scope;
        state.inventory.view = self.view;
        state.mode = Mode::Directory;
    }
}

pub(crate) fn current(state: &State, pending: &PendingCreation) -> bool {
    focused(state, pending)
        && pending.ui_epoch == state.ui_epoch
        && pending.presentation.matches(state)
}

/// The creation still owns the focused tab and pane. This is the concrete
/// destination check shared by presentation authority and input admission;
/// the latter additionally requires terminal mode.
pub(crate) fn focused(state: &State, pending: &PendingCreation) -> bool {
    pending.binding_active
        && state.tabs.active().id == pending.tab
        && state.tab_layout().focused() == pending.pane
}

/// Input is addressed to the focused pane, not to a particular UI epoch. A
/// creation can lose presentation authority (a refresh, a selection change or
/// a fast first keystroke) while its vacant pane is still the one the operator
/// is typing into. Actual navigation still changes the focused tab or pane.
pub(crate) fn input_focused(state: &State, pending: &PendingCreation) -> bool {
    state.mode == Mode::Terminal && focused(state, pending)
}

/// A shell creation (including the durable finalization window) owns the
/// focused vacant pane. Status/pane placeholders use this to say
/// "Starting shell…" instead of the transient Unavailable label.
pub(crate) fn starting_in_focused_pane(state: &State) -> bool {
    let tab = state.tabs.active().id;
    let pane = state.tab_layout().focused();
    let creating =
        state.creations.pending.values().any(|pending| {
            pending.expected.is_none() && pending.tab == tab && pending.pane == pane
        }) || state
            .creations
            .finalizing
            .values()
            .any(|finalization| finalization.tab == Some(tab) && finalization.pane == Some(pane));
    // Binding installs the address before the catalogue publishes the first
    // projection. Keep the creation label through that gap instead of briefly
    // projecting an uncatalogued bound terminal as Unavailable.
    creating
        || (state.status == crate::startup_constants::STARTING_SHELL
            && state
                .focused_terminal()
                .is_some_and(|address| !state.terminals.contains_key(&address)))
}
