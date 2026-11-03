use crate::{Inventory, View};
use std::collections::BTreeMap;
use yolo_hub_identity::{AgentTarget, HubId, TerminalAddress};
use yolo_hub_terminal_protocol::TerminalSummary;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InventorySelection {
    Terminal(TerminalAddress),
    Agent(AgentTarget),
    Hub(HubId),
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
            View::Agents => self
                .visible_agents(filter, terminals)
                .into_iter()
                .map(InventorySelection::Agent)
                .collect(),
            View::Hubs => self
                .visible_hubs(filter)
                .into_iter()
                .map(InventorySelection::Hub)
                .collect(),
        }
    }

    pub fn selected(&self, session: Option<TerminalAddress>) -> Option<InventorySelection> {
        match self.view {
            View::Terminals => session.map(InventorySelection::Terminal),
            View::Agents => self.selected_agent.map(InventorySelection::Agent),
            View::Hubs => self.selected_hub.map(InventorySelection::Hub),
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
            (View::Terminals, InventorySelection::Terminal(address)) => *session = Some(address),
            (View::Agents, InventorySelection::Agent(address)) => {
                self.selected_agent = Some(address)
            }
            (View::Hubs, InventorySelection::Hub(hub)) => self.selected_hub = Some(hub),
            _ => return false,
        }
        true
    }
}
