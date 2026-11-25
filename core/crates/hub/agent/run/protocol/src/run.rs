//! Durable run identity and its public projections.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;
use yolo_hub_identity::{
    AgentRunId, AgentTarget, HubId, TerminalAddress, TerminalId, WorkspaceAddress,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct AgentRunAddress {
    pub hub_id: HubId,
    pub run_id: AgentRunId,
}

/// Fill each run's `current` from the observed terminal-to-execution pairs.
pub fn annotate_current(runs: &mut [AgentRunSummary], observed: &[(TerminalId, AgentTarget)]) {
    let current: BTreeMap<TerminalId, AgentTarget> = observed.iter().copied().collect();
    for run in runs {
        run.current = run
            .terminal
            .filter(|_| run.state == AgentRunState::Launched)
            .and_then(|terminal| {
                current
                    .get(&terminal.terminal_id)
                    .copied()
                    .filter(|target| target.address.hub_id == terminal.hub_id)
            });
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentRunState {
    /// The run identity exists; the terminal may still be launching.
    Starting,
    /// The terminal was created; the provider binding may still be awaiting.
    Launched,
    /// The Host confirmed the terminal ended or was archived, or the launch was cancelled.
    Stopped,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentRunSummary {
    pub address: AgentRunAddress,
    pub revision: u64,
    pub name: String,
    pub aliases: Vec<String>,
    pub labels: BTreeMap<String, String>,
    pub provider_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<WorkspaceAddress>,
    /// Run that created this one. Lineage is durable metadata, not authority.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<AgentRunAddress>,
    pub terminal: Option<TerminalAddress>,
    pub state: AgentRunState,
    /// Latest observed execution bound to the run's terminal; absent until an
    /// execution is observed and after the terminal's observation is gone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current: Option<AgentTarget>,
    #[serde(with = "yolo_crypto::serde_uuid")]
    pub start_token: Uuid,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentRunSelector {
    Address(AgentRunAddress),
    Name(String),
}

impl AgentRunSelector {
    pub fn matches(&self, run: &AgentRunSummary) -> bool {
        match self {
            Self::Address(address) => run.address == *address,
            Self::Name(name) => run.matches_name(name),
        }
    }
}

impl AgentRunSummary {
    fn matches_name(&self, name: &str) -> bool {
        let folded = name.to_lowercase();
        self.name.to_lowercase() == folded
            || self.aliases.iter().any(|alias| alias.to_lowercase() == folded)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "field", rename_all = "snake_case", deny_unknown_fields)]
pub enum AgentRunFilter {
    Terminal { address: TerminalAddress },
    Name { value: String },
    Parent { address: AgentRunAddress },
    State { is: Vec<AgentRunState> },
    Provider { id: String },
    Label { key: String, value: String },
}

impl AgentRunFilter {
    pub fn matches(&self, run: &AgentRunSummary) -> bool {
        match self {
            Self::Terminal { address } => run.terminal == Some(*address),
            Self::Name { value } => run.matches_name(value),
            Self::Parent { address } => run.parent == Some(*address),
            Self::State { is } => is.contains(&run.state),
            Self::Provider { id } => run.provider_id.eq_ignore_ascii_case(id),
            Self::Label { key, value } => run.labels.get(key) == Some(value),
        }
    }
}
