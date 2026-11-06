use crate::{Inventory, InventorySelection, TreeSelection, View};
use std::collections::BTreeMap;
use yolo_hub_agent_protocol::{Freshness, Lifecycle, Reachability};
use yolo_hub_identity::{RuntimeTarget, TerminalAddress};
use yolo_hub_terminal_protocol::TerminalSummary;
use yolo_tui_lifecycle::TerminalPhase;

impl Inventory {
    pub fn reconcile(
        &mut self,
        filter: &str,
        terminals: &BTreeMap<TerminalAddress, TerminalSummary>,
    ) {
        let agents = self.visible_agents(filter, terminals);
        // A live detail view must not outlive the agent it describes. History
        // keeps the row (and its recovery path) available explicitly.
        if self
            .details
            .is_some_and(|details| !agents.contains(&details))
        {
            self.details = None;
        }
        let visible = self.tree_selections(filter, terminals);
        let current = self.tree_selected.map(tree_value);
        if current.is_none_or(|value| !visible.contains(&value)) {
            let preferred = visible
                .iter()
                .find(|value| matches!(value, InventorySelection::Agent(_)))
                .or_else(|| visible.first())
                .copied();
            self.tree_selected = preferred.and_then(tree_selection);
        }
        self.sync_tree_projections();
    }

    pub fn move_selection(
        &mut self,
        forward: bool,
        filter: &str,
        terminals: &BTreeMap<TerminalAddress, TerminalSummary>,
    ) {
        match self.view {
            View::Terminals => {}
            View::Tree => self.move_tree_selection(forward, filter, terminals),
        }
    }

    pub fn terminal_for_selected(
        &self,
        terminals: &BTreeMap<TerminalAddress, TerminalSummary>,
    ) -> Option<RuntimeTarget> {
        if !self.current || !self.agents_supported {
            return None;
        }
        let target = self.selected_agent?;
        let agent = self.agents.get(&target)?.fields();
        if agent.cursor.target() != target
            || matches!(agent.lifecycle, Lifecycle::Exited { .. })
            || agent.freshness != Freshness::Current
            || agent.reachability != Reachability::Reachable
        {
            return None;
        }
        let session = terminals.get(&agent.terminal.address)?.fields();
        if session.address != agent.terminal.address || session.agent != Some(target) {
            return None;
        }
        let runtime = session.runtime?;
        if runtime.target != agent.terminal
            || !matches!(
                self.phase(agent.terminal.address, session),
                TerminalPhase::Running | TerminalPhase::OutputBlocked
            )
        {
            return None;
        }
        Some(runtime.target)
    }
}

fn reconcile<T: Copy + Eq>(selected: &mut Option<T>, visible: &[T]) {
    if selected.is_none_or(|selected| !visible.contains(&selected)) {
        *selected = visible.first().copied();
    }
}

fn tree_value(selection: TreeSelection) -> InventorySelection {
    match selection {
        TreeSelection::Machine(hub) => InventorySelection::Machine(hub),
        TreeSelection::Agent(target) => InventorySelection::Agent(target),
    }
}

fn tree_selection(value: InventorySelection) -> Option<TreeSelection> {
    match value {
        InventorySelection::Machine(hub) => Some(TreeSelection::Machine(hub)),
        InventorySelection::Agent(target) => Some(TreeSelection::Agent(target)),
        InventorySelection::Terminal(_) => None,
    }
}
