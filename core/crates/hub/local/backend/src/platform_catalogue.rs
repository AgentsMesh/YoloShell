//! Platform data exported through the peer catalogue and aggregated by an
//! entry Hub. Absent platform services yield empty pages.

use crate::LocalBackend;
use yolo_hub_agent_run_protocol::{ListAgentRunsRequest, ListAgentRunsResult};
use yolo_hub_terminal_protocol::PublicError;
use yolo_hub_workspace_protocol::{ListWorkspacesRequest, ListWorkspacesResult};

impl LocalBackend {
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
