use crate::{Inventory, TreeSelection, View};
use std::collections::BTreeMap;
use yolo_hub_identity::{AgentTarget, HubId, TerminalAddress};
use yolo_hub_terminal_protocol::TerminalSummary;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InventorySelection {
    Terminal(TerminalAddress),
    Agent(AgentTarget),
    Machine(HubId),
}

impl Inventory {
    pub fn visible_selections(
        &self,
        filter: &str,
        terminals: &BTreeMap<TerminalAddress, TerminalSummary>,
    ) -> Vec<InventorySelection> {
        match self.view {
            View::Terminals => self
                .visible_terminals(filter, terminals)
                .into_iter()
                .map(InventorySelection::Terminal)
                .collect(),
            View::Tree => self.tree_selections(filter, terminals),
        }
    }

    pub fn selected(&self, session: Option<TerminalAddress>) -> Option<InventorySelection> {
        match self.view {
            View::Terminals => session.map(InventorySelection::Terminal),
            View::Tree => self.tree_selected.map(|selection| match selection {
                TreeSelection::Machine(hub) => InventorySelection::Machine(hub),
                TreeSelection::Agent(target) => InventorySelection::Agent(target),
            }),
        }
    }

    /// Selection changes no scope, writer authority, terminal binding or process.
    pub fn select(
        &mut self,
        value: InventorySelection,
        session: &mut Option<TerminalAddress>,
        filter: &str,
        terminals: &BTreeMap<TerminalAddress, TerminalSummary>,
    ) -> bool {
        if !self.visible_selections(filter, terminals).contains(&value) {
            return false;
        }
        match (self.view, value) {
            (View::Terminals, InventorySelection::Terminal(address)) => {
                *session = Some(address);
                true
            }
            (View::Tree, InventorySelection::Agent(target)) => {
                self.select_tree(TreeSelection::Agent(target), filter, terminals)
            }
            (View::Tree, InventorySelection::Machine(hub)) => {
                self.select_tree(TreeSelection::Machine(hub), filter, terminals)
            }
            _ => return false,
        }
    }
}
