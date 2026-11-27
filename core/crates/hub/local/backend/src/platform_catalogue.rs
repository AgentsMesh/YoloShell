use crate::{
    LocalBackend,
    observation::{AppliedCatalogue, SweepPolicy},
    summary::summary,
};
use std::collections::BTreeMap;
use yolo_hub_agent_protocol::AgentSnapshot;
use yolo_hub_agent_run_protocol::{ListAgentRunsRequest, ListAgentRunsResult};
use yolo_hub_connector_registry::Catalogue;
use yolo_hub_identity::WorkspaceAddress;
use yolo_hub_rpc_protocol::{GetPlatformSnapshotRequest, Reply};
use yolo_hub_terminal_protocol::{PublicError, TerminalSummary};
use yolo_hub_workspace_protocol::{ListWorkspacesRequest, ListWorkspacesResult};
use yolo_terminal_model::{TerminalRuntimeStatus, TerminalView};

impl LocalBackend {
    pub(crate) async fn platform_snapshot(
        &self,
        request: &GetPlatformSnapshotRequest,
    ) -> Result<Reply, PublicError> {
        let (terminals, agents) = {
            let _sweep = self.begin_observation().await;
            let mut applied = self
                .apply_complete_catalogue(false, SweepPolicy::ObserveAgents(None))
                .await?;
            let agents = std::mem::take(&mut applied.agents);
            let terminals = self.terminal_summaries_from_applied(applied, false).await?;
            self.adopt_observed(&agents).await;
            (terminals, agents)
        };
        yolo_hub_local_platform::platform_snapshot(
            self.platform.as_deref(),
            self.hub,
            request,
            terminals,
            agents,
        )
        .await
    }

    pub(crate) async fn terminal_summaries_from_applied(
        &self,
        applied: AppliedCatalogue,
        archived: bool,
    ) -> Result<Vec<TerminalSummary>, PublicError> {
        self.reconcile_runs(&applied.views, archived).await?;
        let views = match applied.views {
            Ok(views) => views,
            Err(error) => return Err(crate::errors::map_error(error)),
        };
        let workspaces =
            yolo_hub_local_platform::terminal_workspaces(self.platform.as_deref()).await?;
        views
            .iter()
            .zip(&applied.projected)
            .map(|(view, projected)| {
                terminal_summary(
                    self,
                    view,
                    projected.as_ref(),
                    &applied.connectors,
                    &workspaces,
                )
            })
            .collect()
    }

    pub(crate) async fn refresh_runs(&self) -> Result<(), PublicError> {
        if self.platform.is_none() {
            return Ok(());
        }
        let _sweep = self.begin_observation().await;
        let views = self.bounded_host_read(self.catalogue_views(false)).await;
        self.reconcile_runs(&views, false).await
    }

    pub(crate) async fn reconcile_runs(
        &self,
        views: &Result<Vec<yolo_terminal_model::TerminalView>, yolo_client::ClientError>,
        includes_archived: bool,
    ) -> Result<(), PublicError> {
        if let (Some(platform), Ok(views)) = (self.platform.as_deref(), views) {
            platform
                .observe_run_terminals(self.hub, views, includes_archived)
                .await?;
        }
        Ok(())
    }

    pub async fn platform_runs(
        &self,
        request: &ListAgentRunsRequest,
    ) -> Result<ListAgentRunsResult, PublicError> {
        self.refresh_runs().await?;
        let mut result = match &self.platform {
            Some(platform) => platform.runs().list(request).await?,
            None => ListAgentRunsResult {
                runs: Vec::new(),
                next: None,
            },
        };
        let observed = self.observed_targets().await;
        yolo_hub_agent_run_protocol::annotate_current(&mut result.runs, &observed);
        Ok(result)
    }

    pub async fn platform_workspaces(
        &self,
        request: &ListWorkspacesRequest,
    ) -> Result<ListWorkspacesResult, PublicError> {
        match &self.platform {
            Some(platform) => platform.workspaces().list(request).await,
            None => Ok(ListWorkspacesResult {
                workspaces: Vec::new(),
                next: None,
            }),
        }
    }
}

pub(crate) fn terminal_summary(
    backend: &LocalBackend,
    view: &TerminalView,
    projected: Option<&AgentSnapshot>,
    connectors: &Catalogue,
    workspaces: &BTreeMap<uuid::Uuid, WorkspaceAddress>,
) -> Result<TerminalSummary, PublicError> {
    let mut fields = summary(backend.hub, view)?.fields().clone();
    fields.workspace = workspaces.get(&view.manifest.terminal_id).copied();
    if let Some(runtime) = fields.runtime.as_mut() {
        runtime.size = crate::inventory::dimensions(backend, view);
    }
    let native = projected
        .filter(|agent| {
            !matches!(
                agent.fields().lifecycle,
                yolo_hub_agent_protocol::Lifecycle::Exited { .. }
            ) && view.runtime.as_ref().is_some_and(|runtime| {
                runtime.runtime_epoch == agent.fields().terminal.runtime_epoch.as_uuid()
                    && !matches!(
                        runtime.status,
                        TerminalRuntimeStatus::Exited | TerminalRuntimeStatus::Lost
                    )
            })
        })
        .map(|agent| agent.cursor().target());
    fields.agent = crate::connector_primary::primary(backend.hub, view, native, connectors);
    TerminalSummary::new(fields)
}
