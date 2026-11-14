mod actions;
mod closure;
mod constants;
pub(crate) use closure::close;
mod events;
pub(crate) mod network;
mod open;
mod queries;
mod readiness;
mod submission;
use crate::State;
pub(crate) use actions::handle;
pub(crate) use constants::TOGGLE_KEY;
pub(crate) use events::{completed, control, not_sent};
pub(crate) use open::toggle;
pub(crate) use queries::{pump, synchronize};
pub(crate) use readiness::input as input_readiness;
use std::{collections::BTreeMap, time::Instant};
#[cfg(test)]
pub(crate) use submission::send as submit_for_test;
use uuid::Uuid;
use yolo_hub_agent_protocol::AgentSnapshot;
use yolo_hub_client_channels::{ControlRequest, RequestScope};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) struct Key {
    tab: Uuid,
    pane: Uuid,
}
impl Key {
    pub fn current(state: &State) -> Self {
        Self {
            tab: state.tabs.active().id,
            pane: state.tab_layout().focused(),
        }
    }
}
pub(crate) struct Entry {
    pub view: yolo_tui_chat::Chat,
    pub touched: Instant,
    pub next: Instant,
    pub older: bool,
    pub submission: Option<Pending>,
}
pub(crate) struct Pending {
    pub request: ControlRequest,
    pub _scope: RequestScope,
}
#[derive(Default)]
pub(crate) struct Chats {
    pub entries: BTreeMap<Key, Entry>,
    pub next_catalog: Option<Instant>,
    pub catalog_invalidated: bool,
}

pub(crate) fn agent(state: &State) -> Option<&AgentSnapshot> {
    if !state.inventory.current {
        return None;
    }
    let terminal = state.terminals.get(&state.focused_terminal()?)?;
    let agent = state.inventory.agent_for_target(terminal.fields().agent?)?;
    (terminal.fields().runtime?.target == agent.fields().terminal
        && matches!(
            agent.fields().lifecycle,
            yolo_hub_agent_protocol::Lifecycle::Running {}
        ))
    .then_some(agent)
}
pub(crate) fn current(state: &State) -> Option<&yolo_tui_chat::Chat> {
    state
        .chats
        .entries
        .get(&Key::current(state))
        .map(|entry| &entry.view)
}
pub(crate) fn disconnected(state: &mut State) {
    for entry in state.chats.entries.values_mut() {
        entry.view.composer.disconnected();
        entry.view.set_mode(yolo_tui_chat::Mode::Terminal);
        entry.next = Instant::now();
    }
}
