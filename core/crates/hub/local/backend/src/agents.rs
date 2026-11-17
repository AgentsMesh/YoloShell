use crate::{LocalBackend, connector_catalogue::ObservationScope, observation::SweepPolicy};
use std::sync::Arc;
use tokio::sync::Mutex;
use yolo_agent_provider_contract::ProjectionPolicies;
use yolo_hub_agent_projection::AgentProjection;
use yolo_hub_agent_protocol::AgentSnapshot;
use yolo_hub_identity::{AgentTarget, TerminalAddress};
use yolo_hub_rpc_protocol::{GetPlatformSnapshotRequest, MAX_CATALOG_PAGE_AGENTS, Method, Reply};
use yolo_hub_terminal_protocol::PublicError;
use yolo_paths::directory::PrivateDirectory;

impl LocalBackend {
    pub fn with_launch_policies(
        mut self,
        launch: yolo_agent_provider_contract::LaunchPolicies,
    ) -> Self {
        self.launch = Some(launch);
        self
    }
    pub(crate) async fn agent_start(
        &self,
        request: &yolo_hub_agent_protocol::AgentStartRequest,
    ) -> Result<Reply, PublicError> {
        yolo_hub_local_platform::agent_start(
            self.hub,
            &self.client,
            self.launch.as_ref(),
            self.platform.as_ref().map(|platform| platform.runs()),
            self.platform.as_ref().map(|platform| platform.workspaces()),
            request,
        )
        .await
    }
    pub fn agents_enabled(&self) -> bool {
        self.agents.is_some()
    }
    pub fn with_agents(
        mut self,
        directory: PrivateDirectory,
        policies: ProjectionPolicies,
    ) -> Result<Self, PublicError> {
        let projection = AgentProjection::open_with_policies(directory, self.hub, policies)
            .map_err(|_| PublicError::Internal)?;
        self.agents = Some(Arc::new(Mutex::new(projection)));
        Ok(self)
    }
    pub(crate) fn supports(&self, method: Method) -> bool {
        match method {
            Method::ListLayouts | Method::GetLayout | Method::UpdateLayout => {
                self.layouts.is_some()
            }
            Method::ResolveConversation | Method::ReadConversation | Method::WatchConversation => {
                self.agents.is_some()
            }
            Method::ListAgents
            | Method::GetAgent
            | Method::GetCurrentAgent
            | Method::AgentControl
            | Method::AgentPrompt => self.agents.is_some() || self.connectors.is_some(),
            Method::GetAgentRun
            | Method::ListAgentRuns
            | Method::UpdateAgentRun
            | Method::AgentStop
            | Method::ListPlatformEvents
            | Method::SubscribeEvents => self.platform.is_some(),
            _ => true,
        }
    }
    /// Catalogue reads gather Host and Worker evidence before the projection is
    /// locked: one slow Worker delays this read by at most its observation
    /// budget and never blocks an unrelated command.
    pub(crate) async fn platform_snapshot(
        &self,
        request: &GetPlatformSnapshotRequest,
    ) -> Result<Reply, PublicError> {
        let terminals = self.terminals(false).await?;
        let agents = match self.observe_agents().await {
            Ok(agents) => agents,
            Err(PublicError::Unsupported) => Vec::new(),
            Err(error) => return Err(error),
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

    pub async fn observe_agents(&self) -> Result<Vec<AgentSnapshot>, PublicError> {
        if self.agents.is_none() && self.connectors.is_none() {
            return Err(PublicError::Unsupported);
        }
        let _sweep = self.begin_observation().await;
        let sweep = self
            .observe_catalogue(ObservationScope::Complete, false)
            .await;
        let agents = self
            .apply_catalogue(sweep, true, SweepPolicy::ObserveAgents(None))
            .await?
            .agents;
        self.adopt_observed(&agents).await;
        Ok(agents)
    }
    pub(crate) async fn list_agents(
        &self,
        after: Option<AgentTarget>,
    ) -> Result<Reply, PublicError> {
        let mut agents = self.observe_agents().await?;
        agents.sort_by_key(|agent| agent.cursor().target());
        agents.retain(|agent| after.is_none_or(|after| agent.cursor().target() > after));
        let more = agents.len() > MAX_CATALOG_PAGE_AGENTS;
        agents.truncate(MAX_CATALOG_PAGE_AGENTS);
        let next = more
            .then(|| agents.last().map(|agent| agent.cursor().target()))
            .flatten();
        Ok(Reply::Agents { agents, next })
    }
    pub(crate) async fn get_agent(&self, target: AgentTarget) -> Result<Reply, PublicError> {
        let Some(projection) = &self.agents else {
            return self
                .observe_agents()
                .await?
                .into_iter()
                .find(|agent| agent.cursor().target() == target)
                .map(|agent| Reply::Agent { agent })
                .ok_or(PublicError::NotFound);
        };
        let _sweep = self.begin_observation().await;
        let terminal = {
            let projection = projection.lock().await;
            projection.terminal_for(target)
        };
        let Some(terminal) = terminal else {
            return Err(PublicError::NotFound);
        };
        let scope = ObservationScope::Terminal(TerminalAddress {
            hub_id: self.hub,
            terminal_id: terminal,
        });
        let sweep = self.observe_catalogue(scope, false).await;
        self.apply_catalogue(sweep, true, SweepPolicy::ObserveAgents(Some(terminal)))
            .await?
            .agents
            .into_iter()
            .find(|agent| agent.cursor().target() == target)
            .map(|agent| Reply::Agent { agent })
            .ok_or(PublicError::NotFound)
    }

    /// Expand run selectors to current executions; empty expansion is NotFound.
    pub async fn expand_wait_runs(
        &self,
        request: &mut yolo_hub_agent_protocol::AgentWaitRequest,
    ) -> Result<(), PublicError> {
        if request.runs.is_empty() {
            return Ok(());
        }
        let platform = self.platform.as_deref().ok_or(PublicError::Unsupported)?;
        let mut terminals = Vec::with_capacity(request.runs.len());
        for selector in &request.runs {
            let run = platform.runs().resolve(selector).await?;
            terminals.push(run.terminal.ok_or(PublicError::Conflict)?);
        }
        let views = self.terminals(false).await?;
        let current: std::collections::BTreeMap<yolo_hub_identity::TerminalId, AgentTarget> = views
            .iter()
            .filter_map(|view| {
                let fields = view.fields();
                fields
                    .agent
                    .map(|agent| (fields.address.terminal_id, agent))
            })
            .collect();
        let mut resolved = Vec::new();
        for terminal in terminals {
            resolved.push(*current.get(&terminal.terminal_id).ok_or(PublicError::NotFound)?);
        }
        request.targets.extend(resolved);
        request.targets.sort();
        request.targets.dedup();
        request.runs.clear();
        if request.targets.is_empty() {
            return Err(PublicError::NotFound);
        }
        if request.targets.len() > yolo_hub_agent_protocol::MAX_AGENT_WAIT_TARGETS {
            return Err(PublicError::InvalidRequest);
        }
        Ok(())
    }
}
