use crate::{Inventory, View};
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
        let hubs = self.visible_hubs(filter);
        // A live detail view must not outlive the agent it describes. History
        // keeps the row (and its recovery path) available explicitly.
        if self
            .details
            .is_some_and(|details| !agents.contains(&details))
        {
            self.details = None;
        }
        reconcile(&mut self.selected_agent, &agents);
        reconcile(&mut self.selected_hub, &hubs);
    }

    pub fn move_selection(
        &mut self,
        forward: bool,
        filter: &str,
        terminals: &BTreeMap<TerminalAddress, TerminalSummary>,
    ) {
        match self.view {
            View::Terminals => {}
            View::Agents => {
                let visible = self.visible_agents(filter, terminals);
                move_selection(&mut self.selected_agent, &visible, forward);
            }
            View::Hubs => {
                let visible = self.visible_hubs(filter);
                move_selection(&mut self.selected_hub, &visible, forward);
            }
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
        if !self.in_scope(target.address.hub_id) {
            return None;
        }
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

fn move_selection<T: Copy + Eq>(selected: &mut Option<T>, visible: &[T], forward: bool) {
    let Some(index) =
        selected.and_then(|selected| visible.iter().position(|item| *item == selected))
    else {
        *selected = visible.first().copied();
        return;
    };
    let next = if forward {
        index.saturating_add(1).min(visible.len().saturating_sub(1))
    } else {
        index.saturating_sub(1)
    };
    *selected = visible.get(next).copied();
}
