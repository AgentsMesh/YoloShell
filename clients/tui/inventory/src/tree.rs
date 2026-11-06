use crate::{Inventory, InventorySelection, constants::*};
use std::collections::{BTreeMap, BTreeSet};
use yolo_hub_identity::{AgentTarget, HubId, TerminalAddress};
use yolo_hub_terminal_protocol::TerminalSummary;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TreeSelection {
    Machine(HubId),
    Agent(AgentTarget),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TreeRowKind {
    Machine(HubId),
    Agent(AgentTarget),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TreeRow {
    pub kind: TreeRowKind,
    pub depth: u8,
    pub expanded: bool,
    pub count: usize,
}

impl Inventory {
    pub fn remember_workspace_machines(&mut self, machines: BTreeSet<HubId>) {
        self.workspace_machines = machines;
    }

    pub fn machine_label(&self, hub: HubId) -> String {
        if let Some(name) = self.local_hub_name(hub) {
            return format!("{THIS_MACHINE_LABEL} {name}");
        }
        let label = self
            .machines
            .get(&hub)
            .and_then(|summary| summary.label.as_deref())
            .map_or_else(|| compact_id(&hub.to_string()).to_owned(), str::to_owned);
        format!("{MACHINE_LABEL} {label}")
    }

    pub fn tree_rows(
        &self,
        filter: &str,
        terminals: &BTreeMap<TerminalAddress, TerminalSummary>,
    ) -> Vec<TreeRow> {
        let matching = self.visible_agents(filter, terminals);
        let all = if normalized(filter).is_empty() {
            matching.clone()
        } else {
            self.visible_agents("", terminals)
        };
        let mut hubs = self.machines.keys().copied().collect::<BTreeSet<_>>();
        hubs.extend(self.agents.keys().map(|target| target.address.hub_id));
        hubs.extend(terminals.keys().map(|address| address.hub_id));
        hubs.extend(self.workspace_machines.iter().copied());
        if let Some(local) = self.local_hub {
            hubs.insert(local);
        }
        let mut ordered = hubs.into_iter().collect::<Vec<_>>();
        ordered.sort_by_key(|hub| {
            (
                Some(*hub) != self.local_hub,
                self.machine_label(*hub).to_lowercase(),
                *hub,
            )
        });
        let filtered = !normalized(filter).is_empty();
        let mut rows = Vec::new();
        for hub in ordered {
            let matches_self = machine_matches(self, hub, filter);
            let matching = matching
                .iter()
                .filter(|target| target.address.hub_id == hub)
                .copied()
                .collect::<Vec<_>>();
            let children = if matches_self && filtered {
                all.iter()
                    .filter(|target| target.address.hub_id == hub)
                    .copied()
                    .collect::<Vec<_>>()
            } else {
                matching
            };
            if filtered && !matches_self && children.is_empty() {
                continue;
            }
            let expanded = !children.is_empty()
                && (filtered || !self.collapsed.contains(&hub));
            rows.push(TreeRow {
                kind: TreeRowKind::Machine(hub),
                depth: 0,
                expanded,
                count: children.len(),
            });
            if expanded {
                rows.extend(children.into_iter().map(|target| TreeRow {
                    kind: TreeRowKind::Agent(target),
                    depth: 1,
                    expanded: false,
                    count: 0,
                }));
            }
        }
        rows
    }

    pub fn tree_selections(
        &self,
        filter: &str,
        terminals: &BTreeMap<TerminalAddress, TerminalSummary>,
    ) -> Vec<InventorySelection> {
        self.tree_rows(filter, terminals)
            .into_iter()
            .map(|row| match row.kind {
                TreeRowKind::Machine(hub) => InventorySelection::Machine(hub),
                TreeRowKind::Agent(target) => InventorySelection::Agent(target),
            })
            .collect()
    }
}

fn machine_matches(inventory: &Inventory, hub: HubId, filter: &str) -> bool {
    let filter = normalized(filter);
    filter.is_empty()
        || inventory.machine_label(hub).to_lowercase().contains(&filter)
        || hub.to_string().contains(&filter)
}

fn normalized(filter: &str) -> String {
    filter.trim().to_lowercase()
}

fn compact_id(value: &str) -> &str {
    value.get(..8).unwrap_or(value)
}
