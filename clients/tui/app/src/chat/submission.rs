use super::{Key, Pending, agent, constants::*};
use crate::{Mode, State, network::Job};
use std::time::Instant;
use tokio::sync::mpsc;
use yolo_builtin_client_contract::ClientCapability;
use yolo_hub_client_channels::{ControlRequest, RequestScope};
use yolo_hub_rpc_protocol::Command;

pub(crate) fn send(state: &mut State, jobs: &mpsc::Sender<Job>) {
    if !state.client_capability_enabled(ClientCapability::Conversations) {
        state.status = crate::client_capabilities::CHAT_UNAVAILABLE.into();
        return;
    }
    if !state.client_capability_enabled(ClientCapability::AgentControl) {
        state.status = crate::client_capabilities::AGENT_CONTROL_UNAVAILABLE.into();
        return;
    }
    let key = Key::current(state);
    if !state.inventory.current || state.chats.catalog_invalidated {
        state.status = REFRESHING.into();
        return;
    }
    if state
        .chats
        .entries
        .get(&key)
        .is_some_and(|entry| entry.view.retired())
    {
        state.status = RETIRED.into();
        return;
    }
    let Some(agent) = agent(state).cloned() else {
        state.status = STALE.into();
        return;
    };
    let target = agent.fields().terminal;
    let Some(generation) = state.control_generation else {
        state.status = NEED_CONTROL.into();
        return;
    };
    let Some(grant) = state.grant(target.address, Instant::now()) else {
        state.status = NEED_CONTROL.into();
        return;
    };
    if state.mode != Mode::Chat
        || grant.fields().target != target
        || grant.fields().principal != state.principal
        || state.is_offline(target.address)
        || state.ended_epoch(target.address) == Some(target.runtime_epoch)
    {
        state.status = STALE.into();
        return;
    }
    let Ok(permit) = jobs.try_reserve() else {
        state.status = crate::constants::QUEUE_FULL_LABEL.into();
        return;
    };
    let Some(entry) = state.chats.entries.get_mut(&key) else {
        return;
    };
    let request = match entry.view.submit(&agent) {
        Ok(request) => request,
        Err(error) => {
            if error == yolo_tui_chat::DraftError::ObservationPending {
                state.status = REFRESHING.into();
                crate::catalog_events::invalidate(state, jobs);
            } else {
                state.status = error.to_string();
            }
            return;
        }
    };
    let scope = RequestScope::default();
    let validity = scope.validity_until(state.writers[&target.address].expires);
    let request = ControlRequest {
        generation: Some(generation),
        destination: target.address.hub_id,
        command: Command::AgentControl { request, grant },
        validity: Some(validity),
    };
    entry.submission = Some(Pending {
        request: request.clone(),
        _scope: scope,
    });
    permit.send(Job::Scoped(request));
}
