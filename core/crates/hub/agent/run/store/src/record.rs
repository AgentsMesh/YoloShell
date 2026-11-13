//! Persisted run records and their validation.

use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;
use yolo_hub_agent_run_protocol::{
    AGENT_RUN_PROTOCOL_SCHEMA_VERSION, AgentRunAddress, AgentRunFilter, AgentRunState,
    AgentRunSummary, FIRST_AGENT_RUN_REVISION, validate_aliases, validate_labels, validate_name,
    validate_provider_id,
};
use yolo_hub_identity::{AgentRunId, HubId, TerminalAddress, WorkspaceAddress};
use yolo_hub_terminal_protocol::PublicError;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RunRecord {
    pub(crate) schema_version: u16,
    pub(crate) hub_id: HubId,
    pub(crate) runs: Vec<StoredRun>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct StoredRun {
    pub(crate) address: AgentRunAddress,
    pub(crate) revision: u64,
    pub(crate) name: String,
    pub(crate) aliases: Vec<String>,
    pub(crate) labels: BTreeMap<String, String>,
    pub(crate) provider_id: String,
    #[serde(default)]
    pub(crate) workspace: Option<WorkspaceAddress>,
    #[serde(default)]
    pub(crate) parent: Option<AgentRunAddress>,
    pub(crate) terminal: Option<TerminalAddress>,
    pub(crate) state: AgentRunState,
    pub(crate) start_token: Uuid,
    pub(crate) created_at_ms: i64,
    pub(crate) updated_at_ms: i64,
}

#[derive(Clone, Debug)]
pub struct ReserveAgentRun {
    pub run_id: Uuid,
    pub name: String,
    pub provider_id: String,
    pub workspace: Option<WorkspaceAddress>,
    pub parent: Option<AgentRunAddress>,
    pub start_token: Uuid,
}

impl RunRecord {
    pub(crate) fn empty(hub_id: HubId) -> Self {
        Self {
            schema_version: AGENT_RUN_PROTOCOL_SCHEMA_VERSION,
            hub_id,
            runs: Vec::new(),
        }
    }

    pub(crate) fn validate(&self, hub_id: HubId) -> Result<()> {
        ensure!(
            self.schema_version == AGENT_RUN_PROTOCOL_SCHEMA_VERSION && self.hub_id == hub_id,
            "agent run record identity is invalid"
        );
        let mut ids = BTreeSet::new();
        let mut names = BTreeSet::new();
        let mut tokens = BTreeSet::new();
        for run in &self.runs {
            ensure!(run.address.hub_id == hub_id, "agent run origin mismatch");
            ensure!(
                run.workspace.is_none_or(|workspace| workspace.hub_id == hub_id),
                "agent run workspace origin mismatch"
            );
            if let Some(parent) = run.parent {
                ensure!(parent.hub_id == hub_id, "agent run parent origin mismatch");
                // Insertion order proves the parent already existed, so a
                // persisted lineage cannot contain a cycle.
                ensure!(
                    ids.contains(&parent.run_id),
                    "agent run parent must precede its child"
                );
            }
            ensure!(ids.insert(run.address.run_id), "duplicate agent run id");
            ensure!(tokens.insert(run.start_token), "duplicate start token");
            ensure!(run.revision >= FIRST_AGENT_RUN_REVISION, "invalid revision");
            validate_name(&run.name).map_err(|_| anyhow::anyhow!("invalid run name"))?;
            validate_aliases(&run.aliases).map_err(|_| anyhow::anyhow!("invalid aliases"))?;
            validate_labels(&run.labels).map_err(|_| anyhow::anyhow!("invalid labels"))?;
            validate_provider_id(&run.provider_id)
                .map_err(|_| anyhow::anyhow!("invalid provider"))?;
            for name in std::iter::once(&run.name).chain(run.aliases.iter()) {
                ensure!(
                    names.insert(name.to_lowercase()),
                    "duplicate agent run name or alias"
                );
            }
        }
        Ok(())
    }

    pub(crate) fn find(&self, address: AgentRunAddress) -> Option<&StoredRun> {
        self.runs.iter().find(|run| run.address == address)
    }

    pub(crate) fn find_name(&self, name: &str) -> Option<&StoredRun> {
        let folded = name.to_lowercase();
        self.runs.iter().find(|run| {
            run.name.to_lowercase() == folded
                || run
                    .aliases
                    .iter()
                    .any(|alias| alias.to_lowercase() == folded)
        })
    }

    pub(crate) fn find_token(&self, token: Uuid) -> Option<&StoredRun> {
        self.runs.iter().find(|run| run.start_token == token)
    }
}

pub(crate) fn stored_run(
    hub_id: HubId,
    request: ReserveAgentRun,
) -> Result<StoredRun, PublicError> {
    let now = now_ms();
    Ok(StoredRun {
        address: AgentRunAddress {
            hub_id,
            run_id: AgentRunId::from_uuid(request.run_id)
                .map_err(|_| PublicError::InvalidRequest)?,
        },
        revision: FIRST_AGENT_RUN_REVISION,
        name: request.name,
        aliases: Vec::new(),
        labels: BTreeMap::new(),
        provider_id: request.provider_id,
        workspace: request.workspace,
        parent: request.parent,
        terminal: None,
        state: AgentRunState::Starting,
        start_token: request.start_token,
        created_at_ms: now,
        updated_at_ms: now,
    })
}

pub(crate) fn matches_filter(run: &StoredRun, filter: Option<&AgentRunFilter>) -> bool {
    // One filter policy for local, federated and plugin listings.
    filter.is_none_or(|filter| filter.matches(&summary(run)))
}

pub(crate) fn summary(run: &StoredRun) -> AgentRunSummary {
    AgentRunSummary {
        address: run.address,
        revision: run.revision,
        name: run.name.clone(),
        aliases: run.aliases.clone(),
        labels: run.labels.clone(),
        provider_id: run.provider_id.clone(),
        workspace: run.workspace,
        parent: run.parent,
        terminal: run.terminal,
        state: run.state,
        current: None,
        start_token: run.start_token,
        created_at_ms: run.created_at_ms,
        updated_at_ms: run.updated_at_ms,
    }
}

pub(crate) fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}
