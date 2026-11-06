use crate::{Mode, State, events::enqueue, interrupt_constants::*, network::Job};
use tokio::sync::mpsc;
use uuid::Uuid;
use yolo_hub_agent_protocol::{AgentAction, AgentSnapshot, Capability};
use yolo_hub_rpc_protocol::Command;
use yolo_hub_terminal_protocol::LeaseGrant;
use yolo_tui_inventory::View;

pub(crate) fn start(state: &mut State, jobs: &mpsc::Sender<Job>) {
    let selected = state
        .inventory
        .selected_agent
        .and_then(|address| state.inventory.agents.get(&address));
    if let Some(entry) = selected
        .and_then(|agent| state.interrupts.get(agent.cursor().target()))
        .filter(|entry| entry.phase.blocks())
    {
        state.status = entry.phase.label().into();
        return;
    }
    if let Some(target) = selected.map(|agent| agent.cursor().target()) {
        state.interrupts.dismiss_completed(target);
    }
    let (agent, grant, generation) = match eligible(state) {
        Ok(selection) => selection,
        Err(message) => {
            state.status = message.into();
            return;
        }
    };
    if !state.interrupts.has_capacity() {
        state.status = CAPACITY.into();
        return;
    }
    let Some(request) =
        crate::control_authority::request(&agent, AgentAction::Interrupt {})
    else {
        state.status = UNAVAILABLE.into();
        return;
    };
    let command = Command::AgentControl {
        request: request.clone(),
        grant,
    };
    if enqueue(
        state,
        jobs,
        Job::Call {
            generation: Some(generation),
            origin: request.fields().target.address.hub_id,
            command,
        },
    ) {
        state.interrupts.insert(request, grant, generation);
        state.status = SENDING.into();
    } else {
        state.status = NOT_SENT.into();
    }
}

fn eligible(state: &State) -> Result<(AgentSnapshot, LeaseGrant, Uuid), &'static str> {
    if state.mode != Mode::Inventory
        || state.inventory.view != View::Tree
        || state.inventory.details.is_some()
    {
        return Err(SELECT_AGENT);
    }
    let address = state.inventory.selected_agent.ok_or(SELECT_AGENT)?;
    if !state
        .inventory
        .visible_agents(&state.filter, &state.terminals)
        .contains(&address)
    {
        return Err(SELECT_AGENT);
    }
    let agent = state.inventory.agents.get(&address).ok_or(SELECT_AGENT)?;
    let target = state
        .inventory
        .terminal_for_selected(&state.terminals)
        .ok_or(STALE)?;
    if agent.fields().capabilities.interrupt != (Capability::Available {}) {
        return Err(UNAVAILABLE);
    }
    let (grant, generation) = match crate::control_authority::writer(state, target) {
        Ok(authority) => authority,
        Err(crate::control_authority::WriterError::Unavailable) => {
            return Err(crate::client_capabilities::AGENT_CONTROL_UNAVAILABLE);
        }
        Err(crate::control_authority::WriterError::Stale) => return Err(STALE),
        Err(crate::control_authority::WriterError::TakeControl) => return Err(TAKE_CONTROL),
    };
    Ok((agent.clone(), grant, generation))
}

pub(crate) fn status(state: &State) -> String {
    if state.mode != Mode::Inventory || state.inventory.view != View::Tree {
        return state.status.clone();
    }
    if let Some(entry) = state
        .inventory
        .selected_agent
        .and_then(|address| state.inventory.agents.get(&address))
        .and_then(|agent| state.interrupts.get(agent.cursor().target()))
    {
        return entry.phase.label().into();
    }
    if eligible(state).is_ok() {
        state.status.clone() + STATUS_SEPARATOR + HINT
    } else {
        state.status.clone()
    }
}
