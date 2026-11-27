//! Bounded platform snapshot: one read-only document for initial reconciliation
//! of terminals, agent observations, runs, workspaces and reachable hubs.

use serde::{Deserialize, Serialize};
use yolo_hub_agent_protocol::AgentSnapshot;
use yolo_hub_agent_run_protocol::AgentRunSummary;
use yolo_hub_terminal_protocol::{PublicError, TerminalSummary};
use yolo_hub_workspace_protocol::WorkspaceSummary;

use crate::HubSummary;

pub const PLATFORM_SNAPSHOT_SCHEMA_VERSION: u16 = 1;
pub const MAX_PLATFORM_SNAPSHOT_SECTION: u16 = 128;
pub const MAX_PLATFORM_SNAPSHOT_HUBS: usize = 64;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GetPlatformSnapshotFields {
    pub schema_version: u16,
    pub terminal_limit: u16,
    pub agent_limit: u16,
    pub run_limit: u16,
    pub workspace_limit: u16,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    try_from = "GetPlatformSnapshotFields",
    into = "GetPlatformSnapshotFields"
)]
pub struct GetPlatformSnapshotRequest(GetPlatformSnapshotFields);

impl GetPlatformSnapshotRequest {
    pub fn new(fields: GetPlatformSnapshotFields) -> Result<Self, PublicError> {
        fields.try_into()
    }

    pub fn fields(&self) -> &GetPlatformSnapshotFields {
        &self.0
    }
}

impl TryFrom<GetPlatformSnapshotFields> for GetPlatformSnapshotRequest {
    type Error = PublicError;

    fn try_from(fields: GetPlatformSnapshotFields) -> Result<Self, Self::Error> {
        if fields.schema_version != PLATFORM_SNAPSHOT_SCHEMA_VERSION {
            return Err(PublicError::UnsupportedSchema);
        }
        for limit in [
            fields.terminal_limit,
            fields.agent_limit,
            fields.run_limit,
            fields.workspace_limit,
        ] {
            if limit == 0 || limit > MAX_PLATFORM_SNAPSHOT_SECTION {
                return Err(PublicError::InvalidRequest);
            }
        }
        Ok(Self(fields))
    }
}

impl From<GetPlatformSnapshotRequest> for GetPlatformSnapshotFields {
    fn from(value: GetPlatformSnapshotRequest) -> Self {
        value.0
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformSnapshotTruncation {
    pub hubs: bool,
    pub terminals: bool,
    pub agents: bool,
    pub runs: bool,
    pub workspaces: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformSnapshotFields {
    pub schema_version: u16,
    pub generated_at_ms: i64,
    pub hubs: Vec<HubSummary>,
    pub terminals: Vec<TerminalSummary>,
    pub agents: Vec<AgentSnapshot>,
    pub runs: Vec<AgentRunSummary>,
    pub workspaces: Vec<WorkspaceSummary>,
    pub truncated: PlatformSnapshotTruncation,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "PlatformSnapshotFields", into = "PlatformSnapshotFields")]
pub struct PlatformSnapshot(PlatformSnapshotFields);

impl PlatformSnapshot {
    pub fn new(fields: PlatformSnapshotFields) -> Result<Self, PublicError> {
        fields.try_into()
    }

    pub fn fields(&self) -> &PlatformSnapshotFields {
        &self.0
    }
}

impl TryFrom<PlatformSnapshotFields> for PlatformSnapshot {
    type Error = PublicError;

    fn try_from(fields: PlatformSnapshotFields) -> Result<Self, Self::Error> {
        if fields.schema_version != PLATFORM_SNAPSHOT_SCHEMA_VERSION || fields.generated_at_ms < 0 {
            return Err(PublicError::InvalidMetadata);
        }
        let section = MAX_PLATFORM_SNAPSHOT_SECTION as usize;
        if fields.hubs.len() > MAX_PLATFORM_SNAPSHOT_HUBS
            || fields.terminals.len() > section
            || fields.agents.len() > section
            || fields.runs.len() > section
            || fields.workspaces.len() > section
        {
            return Err(PublicError::InvalidMetadata);
        }
        Ok(Self(fields))
    }
}

impl From<PlatformSnapshot> for PlatformSnapshotFields {
    fn from(value: PlatformSnapshot) -> Self {
        value.0
    }
}
