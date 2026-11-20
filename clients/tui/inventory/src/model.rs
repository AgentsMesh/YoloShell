use std::collections::{BTreeMap, BTreeSet};
use yolo_hub_agent_protocol::AgentSnapshot;
use yolo_hub_identity::{AgentTarget, HubId, RuntimeEpoch, TerminalAddress};
use yolo_hub_rpc_protocol::HubSummary;
use yolo_hub_terminal_protocol::{TerminalSummary, TerminalSummaryFields};
use yolo_hub_workspace_protocol::{WorkspaceAddress, WorkspaceSummary};
use yolo_tui_lifecycle::TerminalPhase;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum View {
    Terminals,
    #[default]
    Agents,
    Hubs,
}

#[derive(Debug, Default)]
pub struct Inventory {
    pub view: View,
    pub scope: Option<HubId>,
    /// The default scope contains live or unconfirmed resources. History is an
    /// explicit observation scope and never changes Hub records itself.
    pub history: bool,
    pub details: Option<AgentTarget>,
    pub agents: BTreeMap<AgentTarget, AgentSnapshot>,
    pub hubs: BTreeMap<HubId, HubSummary>,
    pub selected_agent: Option<AgentTarget>,
    pub selected_hub: Option<HubId>,
    pub local_hub: Option<HubId>,
    pub local_hub_name: Option<String>,
    pub agents_supported: bool,
    pub hubs_supported: bool,
    pub current: bool,
    pub attention: BTreeSet<TerminalAddress>,
    /// Optional workspace scope for terminals and agents; None shows all.
    pub workspace: Option<WorkspaceAddress>,
    pub workspace_names: BTreeMap<WorkspaceAddress, String>,
    /// Runtime epochs this client observed exit before the catalogue page
    /// caught up. Keeping the epoch lets one projection answer both lived and
    /// reused addresses; history still exposes the records.
    pub observed_ended: BTreeMap<TerminalAddress, RuntimeEpoch>,
}

impl Inventory {
    pub fn agent_for_target(&self, target: AgentTarget) -> Option<&AgentSnapshot> {
        self.agents.get(&target)
    }

    /// None is unsupported, not an empty supported catalogue. Transport failure must
    /// instead retain these maps and set current=false; selection remains caller-owned.
    pub fn replace(&mut self, agents: Option<Vec<AgentSnapshot>>, hubs: Option<Vec<HubSummary>>) {
        self.agents_supported = agents.is_some();
        self.hubs_supported = hubs.is_some();
        self.agents = agents
            .unwrap_or_default()
            .into_iter()
            .map(|agent| (agent.cursor().target(), agent))
            .collect();
        self.hubs = hubs
            .unwrap_or_default()
            .into_iter()
            .map(|hub| (hub.hub_id, hub))
            .collect();
        if self
            .details
            .is_some_and(|target| !self.agents.contains_key(&target))
        {
            self.details = None;
        }
    }

    pub(crate) fn in_scope(&self, hub: HubId) -> bool {
        self.scope.is_none_or(|scope| scope == hub)
    }

    /// Name the Hub reached through a local Unix endpoint. Remote/federated
    /// Hubs keep the compact identity fallback because the wire summary does
    /// not carry a host name.
    pub fn set_local_hub(&mut self, hub: HubId, name: Option<String>) {
        self.local_hub = Some(hub);
        self.local_hub_name = name.filter(|name| !name.is_empty());
    }

    pub fn local_hub_name(&self, hub: HubId) -> Option<&str> {
        (self.local_hub == Some(hub))
            .then_some(self.local_hub_name.as_deref())
            .flatten()
    }

    /// Replaces the client's locally observed exits. The catalogue remains the
    /// source of terminal records; this only refines liveness between pages.
    pub fn observe_ended(&mut self, ended: BTreeMap<TerminalAddress, RuntimeEpoch>) {
        self.observed_ended = ended;
    }

    /// The one terminal-liveness projection used by every inventory consumer.
    pub fn phase(&self, address: TerminalAddress, fields: &TerminalSummaryFields) -> TerminalPhase {
        yolo_tui_lifecycle::terminal_phase(fields, self.observed_ended.get(&address).copied())
    }

    pub fn toggle_history(&mut self) {
        self.history = !self.history;
        self.details = None;
    }
}

impl Inventory {
    /// Describes the active workspace scope using the cached name, a member
    /// terminal title or the address. The count stays the catalogue count.
    pub fn workspace_status(
        &self,
        terminals: &BTreeMap<TerminalAddress, TerminalSummary>,
    ) -> String {
        let Some(workspace) = self.workspace else {
            return crate::constants::NO_WORKSPACE_FILTER_LABEL.into();
        };
        let members = terminals
            .values()
            .filter(|terminal| terminal.fields().workspace == Some(workspace))
            .count();
        let name = self
            .workspace_names
            .get(&workspace)
            .cloned()
            .or_else(|| {
                terminals
                    .values()
                    .find(|terminal| terminal.fields().workspace == Some(workspace))
                    .map(|terminal| terminal.display_title().to_owned())
            })
            .unwrap_or_else(|| workspace.workspace_id.to_string());
        format!("Workspace filter: {name} ({members} terminals)")
    }
}

impl Inventory {
    pub fn replace_workspaces(&mut self, workspaces: Vec<WorkspaceSummary>) {
        self.workspace_names = workspaces
            .into_iter()
            .map(|workspace| (workspace.address, workspace.name))
            .collect();
    }
}
