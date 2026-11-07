use super::{Key, agent};
use crate::{State, network::Job};
use std::time::Instant;
use tokio::sync::mpsc;
use yolo_builtin_client_contract::ClientCapability;
use yolo_tui_chat::InputReadiness;

pub(crate) fn input(state: &State) -> InputReadiness {
    if !state.client_capability_enabled(ClientCapability::AgentControl)
        || !state.client_capability_enabled(ClientCapability::Conversations)
    {
        return InputReadiness::Unavailable;
    }
    if !state.inventory.current || state.chats.catalog_invalidated {
        return InputReadiness::AwaitingObservation;
    }
    let Some(agent) = agent(state) else {
        return InputReadiness::Unavailable;
    };
    let target = agent.fields().terminal;
    let Some(grant) = state.grant(target.address, Instant::now()) else {
        return InputReadiness::Unavailable;
    };
    if grant.fields().target != target
        || grant.fields().principal != state.principal
        || state.is_offline(target.address)
        || state.ended_epoch(target.address) == Some(target.runtime_epoch)
    {
        return InputReadiness::Unavailable;
    }
    state
        .chats
        .entries
        .get(&Key::current(state))
        .map_or(InputReadiness::Unavailable, |entry| {
            entry.view.input_readiness(agent)
        })
}

pub(super) fn resolve_status(state: &mut State) {
    if state.mode != crate::Mode::Chat {
        return;
    }
    let ready = input(state) == InputReadiness::Ready;
    let locked = state
        .chats
        .entries
        .get(&Key::current(state))
        .is_some_and(|entry| entry.view.composer.locked());
    if state.status == yolo_tui_chat::DRAFT_LOCKED && !locked {
        state.status.clear();
    } else if state.status == yolo_tui_chat::DRAFT_UNAVAILABLE && ready {
        state.status.clear();
    }
}

pub(super) fn refresh(state: &mut State, jobs: &mpsc::Sender<Job>) {
    if state.mode != crate::Mode::Chat
        || state.control_generation.is_none()
        || !state.client_capability_enabled(ClientCapability::AgentObservation)
    {
        return;
    }
    if state.chats.catalog_invalidated {
        state.chats.catalog_invalidated = false;
        crate::catalog_events::invalidate(state, jobs);
        state.chats.next_catalog = Some(Instant::now() + super::constants::POLL);
        return;
    }
    if state
        .chats
        .entries
        .get(&Key::current(state))
        .is_none_or(|entry| entry.view.retired() || entry.view.composer.request().is_none())
    {
        return;
    }
    let now = Instant::now();
    if state.chats.next_catalog.is_some_and(|next| next > now) {
        return;
    }
    // Transcript updates and delivery receipts are not agent readiness evidence.
    crate::catalog_events::refresh(state, jobs);
    state.chats.next_catalog = Some(now + super::constants::POLL);
}
