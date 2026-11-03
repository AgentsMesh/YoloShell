//! Platform data exported through the peer catalogue and aggregated by an
//! entry Hub. Absent platform services yield empty pages.

use crate::LocalBackend;
use yolo_hub_agent_run_protocol::{ListAgentRunsRequest, ListAgentRunsResult};
use yolo_hub_terminal_protocol::PublicError;
use yolo_hub_workspace_protocol::{ListWorkspacesRequest, ListWorkspacesResult};

impl LocalBackend {
    pub async fn platform_runs(
        &self,
        request: &ListAgentRunsRequest,
    ) -> Result<ListAgentRunsResult, PublicError> {
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
