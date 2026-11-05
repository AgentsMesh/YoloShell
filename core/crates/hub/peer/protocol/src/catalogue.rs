use crate::{PeerError, Purpose, Signed, constants::*};
use serde::{Deserialize, Serialize};
use yolo_hub_agent_protocol::AgentSnapshot;
use yolo_hub_agent_run_protocol::{
    AgentRunSummary, validate_aliases, validate_labels, validate_name, validate_provider_id,
};
use yolo_hub_delegation::{DelegationChain, PeerIdentity};
use yolo_hub_routing::Right;
use yolo_hub_terminal_protocol::TerminalSummary;
use yolo_hub_workspace_protocol::WorkspaceSummary;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Catalogue {
    #[serde(default)]
    pub schema_version: u16,
    pub revision: u64,
    pub policy_epoch: u64,
    pub issued_at_ms: u64,
    pub expires_at_ms: u64,
    pub terminals: Vec<TerminalSummary>,
    pub agents: Vec<AgentSnapshot>,
    /// Bounded durable orchestration identities for peer read aggregation.
    #[serde(default)]
    pub runs: Vec<AgentRunSummary>,
    /// Bounded workspace metadata for peer read aggregation.
    #[serde(default)]
    pub workspaces: Vec<WorkspaceSummary>,
}

impl Catalogue {
    pub fn validate(&self, origin: PeerIdentity) -> Result<(), PeerError> {
        let resources = self.terminals.len()
            + self.agents.len()
            + self.runs.len()
            + self.workspaces.len();
        if self.schema_version != CATALOGUE_SCHEMA_VERSION
            || self.revision < FIRST_REVISION
            || self.policy_epoch < FIRST_REVISION
            || self.expires_at_ms <= self.issued_at_ms
            || self.expires_at_ms - self.issued_at_ms > MAX_CATALOGUE_LIFETIME_MS
            || resources > MAX_CATALOGUE_RESOURCES
            || self
                .terminals
                .iter()
                .any(|session| session.fields().address.hub_id != origin.hub())
            || self
                .agents
                .iter()
                .any(|agent| agent.cursor().target().address.hub_id != origin.hub())
            || self.runs.iter().any(|run| !run_valid(run, origin))
            || self
                .workspaces
                .iter()
                .any(|workspace| !workspace_valid(workspace, origin))
            || self
                .terminals
                .windows(2)
                .any(|pair| pair[0].fields().address >= pair[1].fields().address)
            || self
                .agents
                .windows(2)
                .any(|pair| pair[0].cursor().target() >= pair[1].cursor().target())
            || self
                .runs
                .windows(2)
                .any(|pair| pair[0].address >= pair[1].address)
            || self
                .workspaces
                .windows(2)
                .any(|pair| pair[0].address >= pair[1].address)
        {
            return Err(PeerError::Bounds);
        }
        Ok(())
    }
    pub fn is_current(&self, now: u64) -> bool {
        self.issued_at_ms <= now.saturating_add(MAX_CLOCK_SKEW_MS) && now < self.expires_at_ms
    }
}

fn run_valid(run: &AgentRunSummary, origin: PeerIdentity) -> bool {
    run.address.hub_id == origin.hub()
        && run
            .workspace
            .is_none_or(|workspace| workspace.hub_id == origin.hub())
        && run.parent.is_none_or(|parent| parent.hub_id == origin.hub())
        && run
            .terminal
            .is_none_or(|terminal| terminal.hub_id == origin.hub())
        && validate_name(&run.name).is_ok()
        && validate_aliases(&run.aliases).is_ok()
        && validate_labels(&run.labels).is_ok()
        && validate_provider_id(&run.provider_id).is_ok()
}

fn workspace_valid(workspace: &WorkspaceSummary, origin: PeerIdentity) -> bool {
    workspace.address.hub_id == origin.hub()
        && workspace
            .parent
            .is_none_or(|parent| parent.hub_id == origin.hub())
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExportedOrigin {
    pub catalogue: Signed<Catalogue>,
    pub authority: DelegationChain,
    pub reachable: bool,
}

impl ExportedOrigin {
    pub fn verify(
        &self,
        local: PeerIdentity,
        peer: PeerIdentity,
        origin: PeerIdentity,
        now: u64,
    ) -> Result<(), PeerError> {
        self.catalogue.verify(Purpose::Catalogue, origin)?;
        self.catalogue.payload.validate(origin)?;
        // Old observations remain authentic but must be presented as stale.
        self.authority.authorize(
            origin,
            self.catalogue.payload.policy_epoch,
            local,
            now,
            &[Right::Observe].into_iter().collect(),
        )?;
        if self
            .authority
            .links()
            .last()
            .is_none_or(|grant| grant.fields().claims.issuer != peer)
        {
            return Err(PeerError::Route);
        }
        Ok(())
    }
}
