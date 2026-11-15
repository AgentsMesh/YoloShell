use anyhow::Result;
use uuid::Uuid;
use yolo_hub_agent_run_protocol::{AgentRunAddress, AgentRunSelector};
use yolo_hub_identity::{AgentRunId, HubId};

use crate::{constants::INVALID_RUN_ID, reject::rejected};

pub fn run_selector(hub: HubId, value: &str) -> Result<AgentRunSelector> {
    Ok(match Uuid::parse_str(value) {
        Ok(id) => AgentRunSelector::Address(run_address(hub, id)?),
        Err(_) => AgentRunSelector::Name(value.into()),
    })
}

pub fn run_address(hub: HubId, id: Uuid) -> Result<AgentRunAddress> {
    Ok(AgentRunAddress {
        hub_id: hub,
        run_id: AgentRunId::from_uuid(id).map_err(|_| rejected(INVALID_RUN_ID))?,
    })
}
