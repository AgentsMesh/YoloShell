//! Public platform operations dispatched outside the grandfathered local
//! backend package, which stays at its recorded production budget. This crate
//! maps public requests to Host client calls and back to public replies; it
//! owns no runtime or permission authority.

mod agent_run;
mod agent_run_dispatch;
mod agent_start;
mod agent_stop;
mod caller_context;
mod creation_request;
mod errors;
mod hello;
mod hubs;
mod launch;
mod recovery_hint;
mod snapshot;
mod terminal_run;
mod workspace;
mod workspace_membership;
mod workspace_reservation;
mod workspace_worktree;

pub use agent_run::AgentRunService;
pub use agent_run_dispatch::agent_run_dispatch;
pub use agent_start::agent_start;
pub use agent_stop::agent_stop;
pub use creation_request::{runtime_agent_start_request, runtime_request, runtime_run_request};
pub use errors::map_error;
pub use hello::hello;
pub use hubs::list_hubs;
pub use recovery_hint::{recovery_hint, safe_termination};
pub use snapshot::{adopt_observed, annotate_current, platform_snapshot};
pub use terminal_run::terminal_run;
pub use workspace::{WorkspaceService, workspace_dispatch};
pub use workspace_membership::{annotate_workspace, terminal_workspaces};
pub use yolo_hub_events_bus::EventBus;
pub use yolo_hub_local_terminal_text::read_terminal_text;

/// Hub-owned platform services that live outside the grandfathered local
/// backend package. The backend keeps one handle and one dispatch call.
pub struct PlatformServices {
    workspaces: WorkspaceService,
    runs: AgentRunService,
    events: yolo_hub_events_bus::EventBus,
    client: yolo_client::Client,
}

impl PlatformServices {
    pub fn shared(
        workspace_directory: yolo_paths::directory::PrivateDirectory,
        run_directory: yolo_paths::directory::PrivateDirectory,
        hub: yolo_hub_identity::HubId,
        client: yolo_client::Client,
    ) -> Result<std::sync::Arc<Self>, yolo_hub_terminal_protocol::PublicError> {
        Ok(std::sync::Arc::new(Self {
            workspaces: WorkspaceService::open(workspace_directory, hub, client.clone())?,
            runs: AgentRunService::open(run_directory, hub)?,
            events: EventBus::new(),
            client,
        }))
    }

    pub fn runs(&self) -> &AgentRunService {
        &self.runs
    }

    pub fn workspaces(&self) -> &WorkspaceService {
        &self.workspaces
    }

    /// Append a confirmed public effect to the bounded event stream. Recording
    /// uses only the command and its reply; no screen or PTY data is involved.
    pub fn record(
        &self,
        command: &yolo_hub_rpc_protocol::Command,
        reply: &yolo_hub_rpc_protocol::Reply,
    ) {
        self.events.record(command, reply);
    }

    /// `None` means the command does not belong to a platform service group;
    /// the backend then continues its own routing.
    pub async fn dispatch(
        &self,
        command: &yolo_hub_rpc_protocol::Command,
    ) -> Option<Result<yolo_hub_rpc_protocol::Reply, yolo_hub_terminal_protocol::PublicError>> {
        use yolo_hub_rpc_protocol::Command;
        match command {
            Command::ListPlatformEvents { request } => {
                Some(Ok(yolo_hub_rpc_protocol::Reply::PlatformEvents {
                    events: self.events.list(request),
                }))
            }
            Command::SubscribeEvents { request } => {
                let deadline = tokio::time::Instant::now()
                    + std::time::Duration::from_millis(u64::from(request.fields().wait_ms));
                Some(Ok(yolo_hub_rpc_protocol::Reply::EventSubscription {
                    subscription: self.events.subscribe(request, deadline).await,
                }))
            }
            Command::CreateWorkspace { .. }
            | Command::UpdateWorkspace { .. }
            | Command::GetWorkspace { .. }
            | Command::ListWorkspaces { .. }
            | Command::CreateWorktree { .. }
            | Command::RemoveWorktree { .. }
            | Command::ListWorktrees { .. } => {
                Some(workspace_dispatch(Some(&self.workspaces), command).await)
            }
            Command::AgentStop { request } => {
                Some(agent_stop(&self.runs, &self.client, request).await)
            }
            Command::GetAgentRun { .. }
            | Command::ListAgentRuns { .. }
            | Command::UpdateAgentRun { .. } => {
                Some(agent_run_dispatch(Some(&self.runs), command).await)
            }
            _ => None,
        }
    }
}

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;
