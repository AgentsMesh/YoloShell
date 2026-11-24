//! Worktree workspace records. The Host owns git effects; the store records
//! the verified result and never creates or removes directories.

use anyhow::Result;
use std::collections::BTreeMap;
use yolo_hub_identity::WorkspaceId;
use yolo_hub_terminal_protocol::PublicError;
use yolo_hub_workspace_protocol::{
    CreateWorktreeRequest, FIRST_WORKSPACE_REVISION, ListWorktreesRequest, WorkspaceAddress,
    WorkspaceKind, WorkspaceSummary,
};

use crate::StoreError;
use crate::record::{StoredWorkspace, now_ms, summary};
use crate::store::WorkspaceStore;

impl WorkspaceStore {
    pub fn create_worktree(
        &mut self,
        request: &CreateWorktreeRequest,
        root: String,
    ) -> Result<WorkspaceSummary, StoreError> {
        self.check()?;
        let fields = request.fields();
        if fields.parent.hub_id != self.record.hub_id {
            return Err(PublicError::OriginMismatch.into());
        }
        let parent = self
            .record
            .find(fields.parent)
            .ok_or(PublicError::NotFound)?;
        if parent.kind != WorkspaceKind::Directory {
            return Err(PublicError::Conflict.into());
        }
        let workspace_id =
            WorkspaceId::from_uuid(fields.workspace_id).map_err(|_| PublicError::InvalidRequest)?;
        let address = WorkspaceAddress {
            hub_id: self.record.hub_id,
            workspace_id,
        };
        if let Some(existing) = self.record.find(address) {
            let same = existing.name == fields.name
                && existing.root == root
                && existing.parent == Some(fields.parent)
                && existing.kind == WorkspaceKind::Worktree;
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
            root,
            kind: WorkspaceKind::Worktree,
            parent: Some(fields.parent),
            labels: BTreeMap::new(),
            terminals: Vec::new(),
            runs: Vec::new(),
            created_at_ms: now,
            updated_at_ms: now,
        };
        self.commit(stored.clone())?;
        Ok(summary(&stored))
    }

    pub fn ensure_removable(&self, address: WorkspaceAddress) -> Result<(), StoreError> {
        self.check()?;
        let workspace = self
            .record
            .find(address)
            .ok_or(PublicError::NotFound)?;
        if workspace.kind != WorkspaceKind::Worktree {
            return Err(PublicError::Conflict.into());
        }
        if !workspace.terminals.is_empty()
            || !workspace.runs.is_empty()
            || self.creation_pending(address)
        {
            return Err(PublicError::Conflict.into());
        }
        if self
            .record
            .workspaces
            .iter()
            .any(|child| child.parent == Some(address))
        {
            return Err(PublicError::Conflict.into());
        }
        Ok(())
    }

    pub fn remove(&mut self, address: WorkspaceAddress) -> Result<(), StoreError> {
        self.ensure_removable(address)?;
        let mut record = self.record.clone();
        record
            .workspaces
            .retain(|candidate| candidate.address != address);
        self.commit_record(record)
    }

    pub fn list_worktrees(
        &self,
        request: &ListWorktreesRequest,
    ) -> Result<(Vec<WorkspaceSummary>, Option<WorkspaceAddress>), StoreError> {
        self.check()?;
        let fields = request.fields();
        let after = fields.after;
        let limit = fields.limit as usize;
        let mut rows: Vec<WorkspaceSummary> = self
            .record
            .workspaces
            .iter()
            .filter(|workspace| workspace.parent == Some(fields.parent))
            .filter(|workspace| after.is_none_or(|after| workspace.address > after))
            .map(summary)
            .collect();
        rows.sort_by_key(|workspace| workspace.address);
        let more = rows.len() > limit;
        rows.truncate(limit);
        let next = more.then(|| rows.last().map(|row| row.address)).flatten();
        Ok((rows, next))
    }
}
