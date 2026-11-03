use crate::{Mode, State};

/// Reconciles tab presentations with terminal liveness.
///
/// A tab view is local presentation, so retiring it never mutates a Hub
/// resource. Ended, lost and archived terminals remain in the catalogue's
/// history scope, where they can be inspected and recovered. Terminal status
/// changes invoke this after they update the client's terminal map.
pub(crate) fn retire_dead(state: &mut State) {
    let gone = crate::tab_binding::gone_addresses(state);
    if gone.is_empty() {
        return;
    }
    for address in &gone {
        state.detach_binding(*address);
    }
    let snapshot = state.tab_snapshot();
    let admission = state.replace_tabs(snapshot);
    if admission.active_affected {
        state.activate_tab_collection();
        // A retired pane can leave the terminal view without a target. Keep the
        // inventory visible so the user can pick a live terminal or recover one.
        if state.mode == Mode::Inventory && state.focused_terminal().is_none() {
            state.sidebar_visible = true;
        }
    }
    state.reconcile_selection();
}
