use anyhow::Result;
use uuid::Uuid;
use yolo_hub_agent_run_protocol::{AgentRunAddress, AgentRunSelector};
use yolo_hub_identity::{AgentRunId, HubId};

use crate::{constants::INVALID_RUN_ID, reject::rejected};

#[derive(Clone, Copy, Debug)]
pub enum RunCursor {
    Entry(Uuid),
    Address(AgentRunAddress),
}

impl std::str::FromStr for RunCursor {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if let Ok(id) = Uuid::parse_str(value) {
            return Ok(Self::Entry(id));
        }
        serde_json::from_str(value)
            .map(Self::Address)
            .map_err(|_| "expected a run UUID or the previous page's JSON next cursor")
    }
}

impl RunCursor {
    pub fn address(self, entry: HubId) -> Result<AgentRunAddress> {
        match self {
            Self::Entry(id) => run_address(entry, id),
            Self::Address(address) => Ok(address),
        }
    }
}

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
