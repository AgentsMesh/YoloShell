use crate::{Inventory, InventorySelection};
use std::collections::BTreeMap;
use yolo_hub_agent_protocol::AgentSnapshot;
use yolo_hub_identity::TerminalAddress;
use yolo_hub_rpc_protocol::HubSummary;
use yolo_hub_terminal_protocol::TerminalSummary;

#[derive(Clone, Debug)]
pub(crate) enum RowEvidence {
    Terminal(TerminalSummary),
    Agent(AgentSnapshot, Option<TerminalSummary>),
    Machine(HubSummary),
}
impl RowEvidence {
    pub fn capture(
        row: InventorySelection,
        inventory: &Inventory,
        terminals: &BTreeMap<TerminalAddress, TerminalSummary>,
    ) -> Option<Self> {
        Some(match row {
            InventorySelection::Terminal(address) => Self::Terminal(terminals.get(&address)?.clone()),
            InventorySelection::Agent(target) => {
                let agent = inventory.agents.get(&target)?;
                Self::Agent(
                    agent.clone(),
                    terminals.get(&agent.fields().terminal.address).cloned(),
                )
            }
            InventorySelection::Machine(hub) => Self::Machine(inventory.machines.get(&hub)?.clone()),
        })
    }

    pub fn matches(
        &self,
        inventory: &Inventory,
        terminals: &BTreeMap<TerminalAddress, TerminalSummary>,
    ) -> bool {
        match self {
            Self::Terminal(expected) => terminals.get(&expected.fields().address) == Some(expected),
            Self::Agent(expected, session) => {
                inventory.agents.get(&expected.cursor().target()) == Some(expected)
                    && terminals.get(&expected.fields().terminal.address) == session.as_ref()
            }
            Self::Machine(expected) => inventory.machines.get(&expected.hub_id) == Some(expected),
        }
    }
}
