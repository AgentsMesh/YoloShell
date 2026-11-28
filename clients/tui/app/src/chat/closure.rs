use super::Key;
use crate::State;
use uuid::Uuid;

/// Explicitly closing a view discards its private draft, never its terminal or agent.
pub(crate) fn close(state: &mut State, tab: Uuid, pane: Option<Uuid>) {
    state
        .chats
        .entries
        .retain(|key, _| key.tab != tab || pane.is_some_and(|pane| key.pane != pane));
}

/// A remote layout can close a view. Keep its draft accessible from any view of the same terminal.
pub(crate) fn recover(state: &mut State, current: Key) {
    if state.chats.entries.contains_key(&current) {
        return;
    }
    let Some(address) = state.focused_terminal() else {
        return;
    };
    let catalog = state.tab_snapshot();
    let previous = state.chats.entries.iter().find_map(|(key, entry)| {
        let exists = catalog.entries().iter().any(|tab| {
            tab.id == key.tab
                && tab
                    .view
                    .layout
                    .panes()
                    .iter()
                    .any(|pane| pane.id == key.pane)
        });
        (!exists && entry.view.binding().runtime.address == address).then_some(*key)
    });
    if let Some(previous) = previous {
        if let Some(entry) = state.chats.entries.remove(&previous) {
            state.chats.entries.insert(current, entry);
        }
    }
}
