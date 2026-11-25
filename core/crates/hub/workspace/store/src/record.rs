//! Persisted workspace records and their public projection.

use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;
use yolo_hub_identity::HubId;
use yolo_hub_workspace_protocol::{
    FIRST_WORKSPACE_REVISION, WORKSPACE_PROTOCOL_SCHEMA_VERSION, WorkspaceAddress, WorkspaceKind,
    WorkspaceSummary, validate_labels, validate_name, validate_root,
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct WorkspaceRecord {
    pub(crate) schema_version: u16,
    pub(crate) hub_id: HubId,
    pub(crate) workspaces: Vec<StoredWorkspace>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct StoredWorkspace {
    pub(crate) address: WorkspaceAddress,
    pub(crate) revision: u64,
    pub(crate) name: String,
    pub(crate) root: String,
    pub(crate) kind: WorkspaceKind,
    #[serde(default)]
    pub(crate) parent: Option<WorkspaceAddress>,
    pub(crate) labels: BTreeMap<String, String>,
    #[serde(default)]
    pub(crate) terminals: Vec<Uuid>,
    #[serde(default)]
    pub(crate) runs: Vec<Uuid>,
    pub(crate) created_at_ms: i64,
    pub(crate) updated_at_ms: i64,
}

impl WorkspaceRecord {
    pub(crate) fn empty(hub_id: HubId) -> Self {
        Self {
            schema_version: WORKSPACE_PROTOCOL_SCHEMA_VERSION,
            hub_id,
            workspaces: Vec::new(),
        }
    }

    pub(crate) fn validate(&self, hub_id: HubId) -> Result<()> {
        ensure!(
            self.schema_version == WORKSPACE_PROTOCOL_SCHEMA_VERSION && self.hub_id == hub_id,
            "workspace record identity is invalid"
        );
        let mut ids = BTreeSet::new();
        let mut names = BTreeSet::new();
        let mut members = MemberIds::default();
        for workspace in &self.workspaces {
            ensure!(
                workspace.address.hub_id == hub_id,
                "workspace origin mismatch"
            );
            ensure!(
                ids.insert(workspace.address.workspace_id),
                "duplicate workspace id"
            );
            ensure!(
                names.insert(workspace.name.to_lowercase()),
                "duplicate workspace name"
            );
            ensure!(
                workspace.revision >= FIRST_WORKSPACE_REVISION,
                "invalid workspace revision"
            );
            validate_name(&workspace.name)
                .map_err(|_| anyhow::anyhow!("invalid workspace name"))?;
            validate_root(&workspace.root)
                .map_err(|_| anyhow::anyhow!("invalid workspace root"))?;
            validate_labels(&workspace.labels)
                .map_err(|_| anyhow::anyhow!("invalid workspace labels"))?;
            match workspace.kind {
                WorkspaceKind::Worktree => ensure!(
                    workspace.parent.is_some(),
                    "worktree workspace lacks a parent"
                ),
                WorkspaceKind::Directory => ensure!(
                    workspace.parent.is_none(),
                    "directory workspace cannot have a parent"
                ),
            }
            let terminals: BTreeSet<_> = workspace.terminals.iter().collect();
            ensure!(
                terminals.len() == workspace.terminals.len() && !terminals.contains(&&Uuid::nil()),
                "duplicate or nil workspace terminal member"
            );
            let runs: BTreeSet<_> = workspace.runs.iter().collect();
            ensure!(
                runs.len() == workspace.runs.len() && !runs.contains(&&Uuid::nil()),
                "duplicate or nil workspace run member"
            );
            for id in terminals {
                let fresh = members.terminals.insert(*id);
                ensure!(fresh, "terminal belongs to more than one workspace");
            }
            for id in runs {
                let fresh = members.runs.insert(*id);
                ensure!(fresh, "run belongs to more than one workspace");
            }
        }
        Ok(())
    }

    pub(crate) fn find(&self, address: WorkspaceAddress) -> Option<&StoredWorkspace> {
        self.workspaces
            .iter()
            .find(|workspace| workspace.address == address)
    }

    pub(crate) fn find_name(&self, name: &str) -> Option<&StoredWorkspace> {
        let folded = name.to_lowercase();
        self.workspaces
            .iter()
            .find(|workspace| workspace.name.to_lowercase() == folded)
    }
}

pub(crate) fn summary(workspace: &StoredWorkspace) -> WorkspaceSummary {
    WorkspaceSummary {
        address: workspace.address,
        revision: workspace.revision,
        name: workspace.name.clone(),
        root: workspace.root.clone(),
        kind: workspace.kind,
        parent: workspace.parent,
        labels: workspace.labels.clone(),
        member_terminals: workspace.terminals.len().try_into().unwrap_or(u32::MAX),
        member_runs: workspace.runs.len().try_into().unwrap_or(u32::MAX),
        capabilities: yolo_hub_workspace_protocol::WorkspaceCapabilities::unsupported(
            "git executor is not configured",
        ),
        created_at_ms: workspace.created_at_ms,
        updated_at_ms: workspace.updated_at_ms,
    }
}

#[derive(Default)]
struct MemberIds {
    terminals: BTreeSet<Uuid>,
    runs: BTreeSet<Uuid>,
}

pub(crate) fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}
