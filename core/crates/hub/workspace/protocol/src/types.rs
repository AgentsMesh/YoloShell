//! Workspace summaries and capability projections.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;
use yolo_hub_identity::{HubId, WorkspaceId};
use yolo_hub_terminal_protocol::PublicError;

pub use yolo_hub_identity::WorkspaceAddress;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceKind {
    Directory,
    Worktree,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum GitCapability {
    Available { version: String },
    Unavailable { reason: String },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum WorktreeCapability {
    Available,
    Unavailable { reason: String },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceCapabilities {
    pub git: GitCapability,
    pub worktree: WorktreeCapability,
}

impl WorkspaceCapabilities {
    pub fn unsupported(reason: &str) -> Self {
        Self {
            git: GitCapability::Unavailable {
                reason: reason.into(),
            },
            worktree: WorktreeCapability::Unavailable {
                reason: reason.into(),
            },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceSelector {
    Address(WorkspaceAddress),
    Name(String),
}

/// Move presentation membership for existing terminals. A run's workspace is a
/// durable creation-time identity and is not rebound here.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "change", rename_all = "snake_case", deny_unknown_fields)]
pub enum MembershipChange {
    Bind {
        #[serde(default)]
        terminals: Vec<Uuid>,
    },
    Unbind {
        #[serde(default)]
        terminals: Vec<Uuid>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceSummary {
    pub address: WorkspaceAddress,
    pub revision: u64,
    pub name: String,
    pub root: String,
    pub kind: WorkspaceKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<WorkspaceAddress>,
    pub labels: BTreeMap<String, String>,
    pub member_terminals: u32,
    pub member_runs: u32,
    pub capabilities: WorkspaceCapabilities,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

pub fn address(hub_id: HubId, workspace_id: Uuid) -> Result<WorkspaceAddress, PublicError> {
    Ok(WorkspaceAddress {
        hub_id,
        workspace_id: WorkspaceId::from_uuid(workspace_id)
            .map_err(|_| PublicError::InvalidRequest)?,
    })
}
