use crate::constants::{ADDRESS_SCHEME, AGENT_RESOURCE, TERMINAL_RESOURCE, WORKSPACE_RESOURCE};
use crate::{AgentId, ExecutionId, HubId, RuntimeEpoch, TerminalId, WorkspaceId};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerminalAddress {
    pub hub_id: HubId,
    pub terminal_id: TerminalId,
}

impl fmt::Display for TerminalAddress {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{ADDRESS_SCHEME}://{}/{TERMINAL_RESOURCE}/{}",
            self.hub_id, self.terminal_id
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceAddress {
    pub hub_id: HubId,
    pub workspace_id: WorkspaceId,
}

impl fmt::Display for WorkspaceAddress {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{ADDRESS_SCHEME}://{}/{WORKSPACE_RESOURCE}/{}",
            self.hub_id, self.workspace_id
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentAddress {
    pub hub_id: HubId,
    pub agent_id: AgentId,
}

impl fmt::Display for AgentAddress {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{ADDRESS_SCHEME}://{}/{AGENT_RESOURCE}/{}",
            self.hub_id, self.agent_id
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeTarget {
    pub address: TerminalAddress,
    pub runtime_epoch: RuntimeEpoch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentTarget {
    pub address: AgentAddress,
    pub execution_id: ExecutionId,
}

impl fmt::Display for AgentTarget {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}/{}", self.address, self.execution_id)
    }
}
