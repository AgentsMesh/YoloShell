use crate::State;
use std::time::Instant;
use uuid::Uuid;
use yolo_builtin_client_contract::ClientCapability;
use yolo_hub_agent_protocol::{
    AGENT_PROTOCOL_SCHEMA_VERSION, AgentAction, AgentControlFields, AgentControlRequest,
    AgentSnapshot,
};
use yolo_hub_identity::RuntimeTarget;
use yolo_hub_terminal_protocol::LeaseGrant;

pub(crate) enum WriterError {
    Unavailable,
    Stale,
    TakeControl,
}

/// The one current-writer gate shared by interrupt and interaction answers.
pub(crate) fn writer(
    state: &State,
    target: RuntimeTarget,
) -> Result<(LeaseGrant, Uuid), WriterError> {
    if !state.client_capability_enabled(ClientCapability::AgentControl) {
        return Err(WriterError::Unavailable);
    }
    if !state.inventory.current {
        return Err(WriterError::Stale);
    }
    if state.is_offline(target.address)
        || state.ended_epoch(target.address) == Some(target.runtime_epoch)
    {
        return Err(WriterError::Stale);
    }
    let generation = state.control_generation.ok_or(WriterError::TakeControl)?;
    let grant = state
        .grant(target.address, Instant::now())
        .ok_or(WriterError::TakeControl)?;
    if grant.fields().target != target || grant.fields().principal != state.principal {
        return Err(WriterError::TakeControl);
    }
    Ok((grant, generation))
}

pub(crate) fn request(
    agent: &AgentSnapshot,
    action: AgentAction,
) -> Option<AgentControlRequest> {
    AgentControlRequest::new(AgentControlFields {
        schema_version: AGENT_PROTOCOL_SCHEMA_VERSION,
        request_id: Uuid::new_v4(),
        target: agent.cursor().target(),
        expected_cursor: agent.cursor(),
        action,
    })
    .ok()
}
