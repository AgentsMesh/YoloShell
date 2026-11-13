use super::{Entry, Key, agent, constants::*};
use crate::{Mode, State};
use std::time::Instant;
use yolo_builtin_client_contract::ClientCapability;

pub(crate) fn toggle(state: &mut State) {
    if state.mode == Mode::Chat {
        state.mode = Mode::Terminal;
        super::synchronize(state);
        state.sync_query_authority();
        return;
    }
    let key = Key::current(state);
    if !state.client_capability_enabled(ClientCapability::Conversations) {
        if state.chats.entries.contains_key(&key) {
            enter(state);
        }
        state.status = crate::client_capabilities::CHAT_UNAVAILABLE.into();
        return;
    }
    super::closure::recover(state, key);
    if !state.inventory.current {
        if state.chats.entries.contains_key(&key) {
            enter(state);
        } else {
            state.status = REFRESHING.into();
        }
        return;
    }
    let binding = agent(state).map(|agent| yolo_conversation_protocol::ConversationBinding {
        agent: agent.cursor().target(),
        runtime: agent.fields().terminal,
    });
    if let Some(entry) = state.chats.entries.get_mut(&key) {
        if (entry.view.retired() || Some(entry.view.binding()) != binding)
            && (!entry.view.composer.text().is_empty() || entry.view.composer.locked())
        {
            entry.view.retire();
            enter(state);
            return;
        }
    }
    let Some(binding) = binding else {
        state.status = NO_AGENT.into();
        return;
    };
    if !state.chats.entries.contains_key(&key) && state.chats.entries.len() >= MAX_VIEWS {
        let evict = state
            .chats
            .entries
            .iter()
            .filter(|(_, entry)| {
                entry.view.composer.text().is_empty() && !entry.view.composer.locked()
            })
            .min_by_key(|(_, entry)| entry.touched)
            .map(|(key, _)| *key);
        let Some(evict) = evict else {
            state.status = CACHE_FULL.into();
            return;
        };
        state.chats.entries.remove(&evict);
    }
    if state
        .chats
        .entries
        .get(&key)
        .is_none_or(|entry| entry.view.retired() || entry.view.binding() != binding)
    {
        let Ok(view) = yolo_tui_chat::Chat::new(binding) else {
            state.status = STALE.into();
            return;
        };
        state.chats.entries.insert(
            key,
            Entry {
                view,
                touched: Instant::now(),
                next: Instant::now(),
                older: false,
                submission: None,
            },
        );
    }
    enter(state);
}

fn enter(state: &mut State) {
    state.mode = Mode::Chat;
    state.prefix = false;
    state.pointer = Default::default();
    crate::selection_actions::cancel(state);
    super::synchronize(state);
    state.sync_query_authority();
}
