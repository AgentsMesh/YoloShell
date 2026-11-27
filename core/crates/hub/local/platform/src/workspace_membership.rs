//! Workspace membership: terminal/run bindings, reverse lookup and reconciled
//! member counts. Metadata and worktree effects remain in `workspace.rs`.

use crate::PlatformServices;
use crate::errors::map_error;
use crate::workspace::WorkspaceService;
use std::collections::{BTreeMap, BTreeSet};
use yolo_hub_terminal_protocol::{PublicError, TerminalSummary};
use yolo_hub_workspace_protocol::{
    ListWorkspacesFields, ListWorkspacesRequest, MembershipChange, UpdateWorkspaceRequest,
    WORKSPACE_PROTOCOL_SCHEMA_VERSION, WorkspaceAddress, WorkspaceSummary,
};
use yolo_hub_workspace_store::StoreError;

impl WorkspaceService {
    pub async fn bind_terminal(
        &self,
        address: WorkspaceAddress,
        terminal_id: uuid::Uuid,
    ) -> Result<(), PublicError> {
        self.store()
            .lock()
            .await
            .bind_terminal(address, terminal_id)
            .map_err(StoreError::into_public)
    }

    pub async fn bind_run(
        &self,
        address: WorkspaceAddress,
        run_id: uuid::Uuid,
    ) -> Result<(), PublicError> {
        self.store()
            .lock()
            .await
            .bind_run(address, run_id)
            .map_err(StoreError::into_public)
    }

    pub async fn terminal_workspaces(
        &self,
    ) -> Result<Vec<(uuid::Uuid, WorkspaceAddress)>, PublicError> {
        self.store()
            .lock()
            .await
            .terminal_workspaces()
            .map_err(StoreError::into_public)
    }

    /// Count only terminals still present in the Host catalogue; runs are
    /// durable members because the run store never deletes them.
    pub async fn apply_counts(
        &self,
        summaries: &mut [WorkspaceSummary],
    ) -> Result<(), PublicError> {
        if summaries.is_empty() {
            return Ok(());
        }
        let addresses: Vec<_> = summaries.iter().map(|summary| summary.address).collect();
        let found = self
            .store()
            .lock()
            .await
            .members_many(&addresses)
            .map_err(StoreError::into_public)?;
        let needs_live = found.iter().any(|members| {
            members
                .as_ref()
                .is_some_and(|(terminals, _)| !terminals.is_empty())
        });
        // A Host read failure falls back to the durable member ids so
        // workspace metadata stays readable during Host maintenance.
        let live: Option<BTreeSet<uuid::Uuid>> = if needs_live {
            self.client()
                .list_sessions(true)
                .await
                .ok()
                .map(|views| views.iter().map(|view| view.manifest.terminal_id).collect())
        } else {
            None
        };
        for (summary, members) in summaries.iter_mut().zip(found) {
            let Some((terminals, runs)) = members else {
                continue;
            };
            summary.member_terminals = match &live {
                Some(live) => terminals.iter().filter(|id| live.contains(id)).count(),
                None => terminals.len(),
            }
            .try_into()
            .unwrap_or(u32::MAX);
            summary.member_runs = runs.len().try_into().unwrap_or(u32::MAX);
        }
        Ok(())
    }

    pub async fn snapshot_page(
        &self,
        limit: u16,
    ) -> Result<(Vec<WorkspaceSummary>, bool), PublicError> {
        let request = ListWorkspacesRequest::new(ListWorkspacesFields {
            schema_version: WORKSPACE_PROTOCOL_SCHEMA_VERSION,
            after: None,
            limit,
        })?;
        let list = self.list(&request).await?;
        Ok((list.workspaces, list.next.is_some()))
    }

    pub async fn exists(&self, address: WorkspaceAddress) -> Result<bool, PublicError> {
        Ok(self
            .store()
            .lock()
            .await
            .get(address)
            .map_err(StoreError::into_public)?
            .is_some())
    }
}

/// Terminal-to-workspace memberships for catalogue presentation.
pub async fn terminal_workspaces(
    platform: Option<&PlatformServices>,
) -> Result<BTreeMap<uuid::Uuid, WorkspaceAddress>, PublicError> {
    let Some(platform) = platform else {
        return Ok(BTreeMap::new());
    };
    Ok(platform
        .workspaces()
        .terminal_workspaces()
        .await?
        .into_iter()
        .collect())
}

/// Join the durable workspace reference into a terminal summary.
pub async fn annotate_workspace(
    platform: Option<&PlatformServices>,
    summary: TerminalSummary,
) -> Result<TerminalSummary, PublicError> {
    let mut fields = summary.fields().clone();
    fields.workspace = terminal_workspaces(platform)
        .await?
        .get(&fields.address.terminal_id.as_uuid())
        .copied();
    TerminalSummary::new(fields)
}

/// Apply a terminal bind/unbind after verifying every bound terminal exists.
/// Unbinding is idempotent. Caller context injected at creation is not
/// rewritten by a later rebind; run membership stays creation-time.
pub(crate) async fn update(
    service: &WorkspaceService,
    request: &UpdateWorkspaceRequest,
) -> Result<WorkspaceSummary, PublicError> {
    if let Some(MembershipChange::Bind { terminals }) = &request.fields().members {
        let known: BTreeSet<uuid::Uuid> = service
            .client()
            .list_sessions(true)
            .await
            .map_err(map_error)?
            .iter()
            .map(|view| view.manifest.terminal_id)
            .collect();
        if terminals.iter().any(|id| !known.contains(id)) {
            return Err(PublicError::NotFound);
        }
    }
    service
        .store()
        .lock()
        .await
        .update(request)
        .map_err(StoreError::into_public)
}
