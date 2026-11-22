use crate::{Publication, constants::*};
use anyhow::{Context, Result};
use yolo_hub_agent_run_protocol::{
    AGENT_RUN_PROTOCOL_SCHEMA_VERSION, ListAgentRunsFields, ListAgentRunsRequest,
    MAX_AGENT_RUN_PAGE,
};
use yolo_hub_local_backend::LocalBackend;
use yolo_hub_peer_protocol::{
    CATALOGUE_SCHEMA_VERSION, Catalogue, FIRST_REVISION, MAX_CATALOGUE_LIFETIME_MS, Purpose,
    Signed,
};
use yolo_hub_state::HubState;
use yolo_hub_workspace_protocol::{
    ListWorkspacesFields, ListWorkspacesRequest, MAX_WORKSPACE_PAGE,
    WORKSPACE_PROTOCOL_SCHEMA_VERSION,
};

impl Publication {
    pub async fn refresh(
        &mut self,
        backend: &LocalBackend,
        state: &HubState,
        now: u64,
    ) -> Result<()> {
        self.verify()?;
        state.verify()?;
        // Peers receive the live catalogue; archived sessions are local
        // history and must not make every refresh an O(all sessions) sweep.
        let mut terminals = match backend.terminals(false).await {
            Ok(terminals) => terminals,
            Err(_) => {
                let mut terminals = self
                    .record
                    .latest
                    .as_ref()
                    .map(|latest| latest.payload.terminals.clone())
                    .unwrap_or_default();
                for session in &mut terminals {
                    session.mark_unreachable();
                }
                terminals
            }
        };
        let mut agents = backend.observe_agents().await?;
        let runs = backend
            .platform_runs(&ListAgentRunsRequest::new(ListAgentRunsFields {
                schema_version: AGENT_RUN_PROTOCOL_SCHEMA_VERSION,
                after: None,
                limit: MAX_AGENT_RUN_PAGE as u16,
                filter: None,
            })?)
            .await?
            .runs;
        let workspaces = backend
            .platform_workspaces(&ListWorkspacesRequest::new(ListWorkspacesFields {
                schema_version: WORKSPACE_PROTOCOL_SCHEMA_VERSION,
                after: None,
                limit: MAX_WORKSPACE_PAGE as u16,
            })?)
            .await?
            .workspaces;
        terminals.sort_by_key(|session| session.fields().address);
        agents.sort_by_key(|agent| agent.cursor().target());
        let revision = self
            .record
            .revision
            .checked_add(FIRST_REVISION)
            .context(EXHAUSTED)?;
        let catalogue = Catalogue {
            schema_version: CATALOGUE_SCHEMA_VERSION,
            revision,
            policy_epoch: self.record.policy_epoch,
            issued_at_ms: now,
            expires_at_ms: now
                .checked_add(MAX_CATALOGUE_LIFETIME_MS)
                .context(EXHAUSTED)?,
            terminals,
            agents,
            runs,
            workspaces,
        };
        catalogue.validate(self.record.identity)?;
        let latest = Signed::new(catalogue, Purpose::Catalogue, state)?;
        let mut record = self.record.clone();
        record.revision = revision;
        record.latest = Some(latest);
        self.namespace.write_json(&self.path, &record)?;
        self.verify()?;
        self.record = record;
        Ok(())
    }
}
