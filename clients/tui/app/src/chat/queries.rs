use super::{Key, agent, constants::*, network::Operation, readiness};
use crate::{Mode, State, network::Job};
use std::time::Instant;
use tokio::sync::mpsc;
use yolo_builtin_client_contract::ClientCapability;
use yolo_tui_chat::{Mode as ChatMode, QueryError, ReadKind};

pub(crate) fn synchronize(state: &mut State) {
    let observable = state.client_capability_enabled(ClientCapability::AgentObservation);
    let readable = state.client_capability_enabled(ClientCapability::Conversations);
    let key = Key::current(state);
    let binding = agent(state).map(|agent| yolo_conversation_protocol::ConversationBinding {
        agent: agent.cursor().target(),
        runtime: agent.fields().terminal,
    });
    for (id, entry) in &mut state.chats.entries {
        let active = *id == key && state.mode == Mode::Chat && readable;
        if *id == key
            && observable
            && state.inventory.current
            && Some(entry.view.binding()) != binding
            && !entry.view.retired()
        {
            entry.view.retire();
        }
        entry.view.set_mode(if active {
            ChatMode::Chat
        } else {
            ChatMode::Terminal
        });
    }
    if let Some(cursor) = observable
        .then(|| agent(state))
        .flatten()
        .map(|agent| agent.cursor())
    {
        if let Some(entry) = state.chats.entries.get_mut(&key) {
            if !entry.view.retired() && entry.view.binding().agent == cursor.target() {
                entry.view.composer.observe(cursor);
            }
        }
    }
    if state.mode == Mode::Chat
        && observable
        && state.inventory.current
        && state
            .chats
            .entries
            .get(&key)
            .is_none_or(|entry| Some(entry.view.binding()) != binding)
    {
        state.status = RETIRED.into();
    } else if state.inventory.current && state.status == REFRESHING {
        state.status = crate::constants::CONNECTED_LABEL.into();
    }
}

pub(crate) fn pump(state: &mut State, jobs: &mpsc::Sender<Job>) {
    synchronize(state);
    readiness::resolve_status(state);
    if !state.client_capability_enabled(ClientCapability::Conversations) {
        return;
    }
    super::readiness::refresh(state, jobs);
    let key = Key::current(state);
    let Some(generation) = state.control_generation else {
        return;
    };
    let Some(binding) = agent(state).map(|agent| yolo_conversation_protocol::ConversationBinding {
        agent: agent.cursor().target(),
        runtime: agent.fields().terminal,
    }) else {
        return;
    };
    let Some(entry) = state.chats.entries.get_mut(&key) else {
        return;
    };
    let now = Instant::now();
    if entry.view.retired()
        || entry.view.binding() != binding
        || entry.view.mode() != ChatMode::Chat
        || now < entry.next
        || entry.view.pending().is_some()
        || entry.view.error() == Some(QueryError::Unsupported)
    {
        return;
    }
    entry.touched = now;
    let query = if entry.view.descriptor().is_none() {
        entry.view.resolve()
    } else if entry.older && entry.view.older_available() {
        entry.older = false;
        entry.view.read(ReadKind::Older)
    } else {
        entry.older = false;
        entry.view.read(ReadKind::Latest)
    };
    let Some(query) = query else { return };
    let operation = Operation {
        key,
        generation,
        query: query.clone(),
    };
    if jobs.try_send(Job::Chat(operation)).is_err() {
        entry.view.fail(&query, QueryError::Disconnected);
        entry.next = now + POLL;
    }
}
