use crate::{Mode, State, events::enqueue, network::Job};
use tokio::sync::mpsc;
use uuid::Uuid;
use yolo_hub_agent_protocol::{AgentAction, AgentSnapshot, Capability, InteractionResponse};
use yolo_hub_rpc_protocol::Command;
use yolo_hub_terminal_protocol::LeaseGrant;
use yolo_tui_inventory::View;

const OPEN_DETAILS: &str = "Open Agent details to answer its interaction";
const NO_INTERACTION: &str = "The selected Agent has no pending interaction";
const UNAVAILABLE: &str = "This Agent cannot answer the interaction";
const STALE: &str = "Refresh the Agent before answering";
const TAKE_CONTROL: &str = "Take control of the terminal before answering";
const SENT: &str = "Answer sent; awaiting result";
const IN_FLIGHT: &str = "Answer already in flight";

/// Answer choice `index` (zero-based) from the details view.
pub(crate) fn choose(state: &mut State, jobs: &mpsc::Sender<Job>, index: usize) -> bool {
    let Some((agent, grant, generation, choice)) = eligible(state, index) else {
        return false;
    };
    let target = agent.cursor().target();
    if !state.respond_pending.insert(target) {
        state.status = IN_FLIGHT.into();
        return false;
    }
    let action = AgentAction::Respond {
        interaction: agent
            .fields()
            .interaction
            .as_ref()
            .expect("eligible interaction")
            .reference(),
        response: InteractionResponse::Choice { choice },
    };
    let Some(request) = crate::control_authority::request(&agent, action) else {
        state.respond_pending.remove(&target);
        state.status = UNAVAILABLE.into();
        return false;
    };
    let command = Command::AgentControl {
        request,
        grant,
    };
    if enqueue(
        state,
        jobs,
        Job::Call {
            generation: Some(generation),
            origin: agent.cursor().target().address.hub_id,
            command,
        },
    ) {
        state.status = SENT.into();
        true
    } else {
        state.respond_pending.remove(&target);
        false
    }
}

fn eligible(state: &mut State, index: usize) -> Option<(AgentSnapshot, LeaseGrant, Uuid, String)> {
    if state.mode != Mode::Inventory
        || state.inventory.view != View::Tree
        || state.inventory.details.is_none()
    {
        state.status = OPEN_DETAILS.into();
        return None;
    }
    let address = state.inventory.details?;
    let agent = state.inventory.agents.get(&address)?.clone();
    let fields = agent.fields();
    let Some(interaction) = fields.interaction.as_ref() else {
        state.status = NO_INTERACTION.into();
        return None;
    };
    if fields.capabilities.respond_interaction != (Capability::Available {}) {
        state.status = UNAVAILABLE.into();
        return None;
    }
    let Some(choice) = interaction.choices().get(index).map(|choice| choice.id.clone()) else {
        state.status = NO_INTERACTION.into();
        return None;
    };
    let (grant, generation) = match crate::control_authority::writer(state, fields.terminal) {
        Ok(authority) => authority,
        Err(crate::control_authority::WriterError::Unavailable) => {
            state.status = crate::client_capabilities::AGENT_CONTROL_UNAVAILABLE.into();
            return None;
        }
        Err(crate::control_authority::WriterError::Stale) => {
            state.status = STALE.into();
            return None;
        }
        Err(crate::control_authority::WriterError::TakeControl) => {
            state.status = TAKE_CONTROL.into();
            return None;
        }
    };
    Some((agent, grant, generation, choice))
}
