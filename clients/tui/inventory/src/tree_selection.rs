use crate::{Inventory, InventorySelection, TreeSelection};
use std::collections::BTreeMap;
use yolo_hub_identity::{HubId, TerminalAddress};
use yolo_hub_terminal_protocol::TerminalSummary;

impl Inventory {
    pub fn select_tree(
        &mut self,
        selection: TreeSelection,
        filter: &str,
        terminals: &BTreeMap<TerminalAddress, TerminalSummary>,
    ) -> bool {
        let value = selection_value(selection);
        if !self.tree_selections(filter, terminals).contains(&value) {
            return false;
        }
        self.set_tree_selection(selection);
        true
    }

    pub fn move_tree_selection(
        &mut self,
        forward: bool,
        filter: &str,
        terminals: &BTreeMap<TerminalAddress, TerminalSummary>,
    ) {
        let visible = self.tree_selections(filter, terminals);
        let current = self
            .tree_selected
            .map(selection_value)
            .and_then(|value| visible.iter().position(|item| *item == value));
        let next = match current {
            Some(index) if forward => index.saturating_add(1).min(visible.len() - 1),
            Some(index) => index.saturating_sub(1),
            None => 0,
        };
        if let Some(value) = visible.get(next) {
            self.set_tree_selection(match value {
                InventorySelection::Machine(hub) => TreeSelection::Machine(*hub),
                InventorySelection::Agent(target) => TreeSelection::Agent(*target),
                InventorySelection::Terminal(_) => return,
            });
        }
    }

    pub fn toggle_machine(&mut self, hub: HubId) {
        if !self.collapsed.remove(&hub) {
            self.collapsed.insert(hub);
        }
    }

    pub fn selected_tree(&self) -> Option<TreeSelection> {
        self.tree_selected
    }

    pub(crate) fn set_tree_selection(&mut self, selection: TreeSelection) {
        self.tree_selected = Some(selection);
        self.sync_tree_projections();
    }

    pub(crate) fn sync_tree_projections(&mut self) {
        match self.tree_selected {
            Some(TreeSelection::Machine(hub)) => {
                self.selected_machine = Some(hub);
                self.selected_agent = None;
            }
            Some(TreeSelection::Agent(target)) => {
                self.selected_agent = Some(target);
                self.selected_machine = Some(target.address.hub_id);
            }
            None => {
                self.selected_agent = None;
                self.selected_machine = None;
            }
        }
    }
}

fn selection_value(selection: TreeSelection) -> InventorySelection {
    match selection {
        TreeSelection::Machine(hub) => InventorySelection::Machine(hub),
        TreeSelection::Agent(target) => InventorySelection::Agent(target),
    }
}
