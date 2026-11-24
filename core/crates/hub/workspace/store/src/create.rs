//! Directory workspace creation and metadata updates.

use anyhow::Result;
use uuid::Uuid;
use yolo_hub_identity::WorkspaceId;
use yolo_hub_terminal_protocol::PublicError;
use yolo_hub_workspace_protocol::{
    CreateWorkspaceRequest, FIRST_WORKSPACE_REVISION, MembershipChange, UpdateWorkspaceRequest,
    WorkspaceAddress, WorkspaceKind, WorkspaceSummary,
};

use crate::StoreError;
use crate::record::{StoredWorkspace, now_ms, summary};
use crate::store::WorkspaceStore;

impl WorkspaceStore {
    pub fn create(
        &mut self,
        request: &CreateWorkspaceRequest,
    ) -> Result<WorkspaceSummary, StoreError> {
        self.check()?;
        let fields = request.fields();
        let workspace_id =
            WorkspaceId::from_uuid(fields.workspace_id).map_err(|_| PublicError::InvalidRequest)?;
        let address = WorkspaceAddress {
            hub_id: self.record.hub_id,
            workspace_id,
        };
        if let Some(existing) = self.record.find(address) {
            let same = existing.name == fields.name
                && existing.root == fields.root
                && existing.labels == fields.labels
                && existing.kind == WorkspaceKind::Directory;
            return if same {
                Ok(summary(existing))
            } else {
                Err(PublicError::Conflict.into())
            };
        }
        if self.record.find_name(&fields.name).is_some() {
            return Err(PublicError::Conflict.into());
        }
        let now = now_ms();
        let stored = StoredWorkspace {
            address,
            revision: FIRST_WORKSPACE_REVISION,
            name: fields.name.clone(),
            root: fields.root.clone(),
            kind: WorkspaceKind::Directory,
            parent: None,
            labels: fields.labels.clone(),
            terminals: Vec::new(),
            runs: Vec::new(),
            created_at_ms: now,
            updated_at_ms: now,
        };
        self.commit(stored.clone())?;
        Ok(summary(&stored))
    }

    pub fn update(
        &mut self,
        request: &UpdateWorkspaceRequest,
    ) -> Result<WorkspaceSummary, StoreError> {
        self.check()?;
        let fields = request.fields();
        if fields.address.hub_id != self.record.hub_id {
            return Err(PublicError::OriginMismatch.into());
        }
        let current = self
            .record
            .find(fields.address)
            .cloned()
            .ok_or(PublicError::NotFound)?;
        if current.revision != fields.expected_revision {
            return Err(PublicError::Conflict.into());
        }
        let name = fields.name.clone().unwrap_or_else(|| current.name.clone());
        if let Some(other) = self.record.find_name(&name) {
            if other.address != current.address {
                return Err(PublicError::Conflict.into());
            }
        }
        let mut terminals = current.terminals;
        if let Some(change) = fields.members.as_ref() {
            match change {
                MembershipChange::Bind {
                    terminals: bind_terminals,
                } => bind_members(
                    &self.record.workspaces,
                    current.address,
                    bind_terminals,
                    &mut terminals,
                )?,
                MembershipChange::Unbind {
                    terminals: unbind_terminals,
                } => terminals.retain(|id| !unbind_terminals.contains(id)),
            }
        }
        let next = StoredWorkspace {
            address: current.address,
            revision: current.revision + 1,
            name,
            root: current.root,
            kind: current.kind,
            parent: current.parent,
            labels: fields.labels.clone().unwrap_or(current.labels),
            terminals,
            runs: current.runs,
            created_at_ms: current.created_at_ms,
            updated_at_ms: now_ms(),
        };
        self.commit(next.clone())?;
        Ok(summary(&next))
    }
}

fn bind_members(
    workspaces: &[StoredWorkspace],
    address: WorkspaceAddress,
    ids: &[Uuid],
    members: &mut Vec<Uuid>,
) -> Result<(), StoreError> {
    for id in ids {
        if members.contains(id) {
            continue;
        }
        let conflict = workspaces.iter().any(|workspace| {
            workspace.address != address && workspace.terminals.contains(id)
        });
        if conflict {
            return Err(PublicError::Conflict.into());
        }
        members.push(*id);
    }
    members.sort_unstable();
    Ok(())
}
