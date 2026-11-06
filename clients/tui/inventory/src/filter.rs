use crate::{Inventory, labels};
use std::collections::BTreeMap;
use yolo_hub_agent_protocol::{Freshness, Lifecycle, Reachability};
use yolo_hub_identity::{AgentTarget, HubId, TerminalAddress};
use yolo_hub_terminal_protocol::TerminalSummary;
use yolo_tui_lifecycle::TerminalPhase;

impl Inventory {
    pub fn visible_terminals(
        &self,
        filter: &str,
        terminals: &BTreeMap<TerminalAddress, TerminalSummary>,
    ) -> Vec<TerminalAddress> {
        let filter = normalized(filter);
        terminals
            .iter()
            .filter(|(address, session)| {
                let fields = session.fields();
                let phase = self.phase(**address, fields);
                self
                        .workspace
                        .is_none_or(|workspace| fields.workspace == Some(workspace))
                    && (self.history || !terminal_is_dead(phase))
                    && matches(
                        &filter,
                        &[
                            &fields.name,
                            session.display_title(),
                            &fields.cwd,
                            &address.to_string(),
                            phase.label(),
                            runtime_search_alias(phase),
                        ],
                    )
            })
            .map(|(address, _)| *address)
            .collect()
    }

    pub fn visible_agents(
        &self,
        filter: &str,
        terminals: &BTreeMap<TerminalAddress, TerminalSummary>,
    ) -> Vec<AgentTarget> {
        if !self.agents_supported {
            return Vec::new();
        }
        let filter = normalized(filter);
        self.agents
            .iter()
            .filter(|(target, snapshot)| {
                let fields = snapshot.fields();
                let session = terminals
                    .get(&fields.terminal.address)
                    .map(TerminalSummary::fields);
                if self.workspace.is_some_and(|workspace| {
                    session.is_none_or(|session| session.workspace != Some(workspace))
                }) {
                    return false;
                }
                let phase = session.map(|session| self.phase(fields.terminal.address, session));
                if !self.history && !agent_is_active(fields, session, phase) {
                    return false;
                }
                matches(
                    &filter,
                    &[
                        &target.to_string(),
                        &fields.terminal.address.to_string(),
                        fields.provider.kind(),
                        fields.provider.display_name().unwrap_or_default(),
                        crate::agent_title(snapshot, terminals),
                        fields.provider.version().unwrap_or_default(),
                        fields
                            .provider
                            .native_session_reference()
                            .unwrap_or_default(),
                        session.map_or("", |session| session.name.as_str()),
                        fields
                            .working_directory
                            .as_ref()
                            .map_or("", |directory| directory.as_str()),
                        labels::lifecycle(fields.lifecycle),
                        labels::activity(fields.activity),
                    ],
                )
            })
            .map(|(target, _)| *target)
            .collect()
    }

    /// Machines is the scope selector itself, so the current scope never hides other machines.
    pub fn visible_machines(&self, filter: &str) -> Vec<HubId> {
        if !self.machines_supported {
            return Vec::new();
        }
        let filter = normalized(filter);
        self.machines
            .iter()
            .filter(|(hub, summary)| {
                matches(
                    &filter,
                    &[
                        &hub.to_string(),
                        labels::observed_reachability(self.current, summary.reachable),
                    ],
                )
            })
            .map(|(hub, _)| *hub)
            .collect()
    }
}

fn runtime_search_alias(phase: TerminalPhase) -> &'static str {
    match phase {
        TerminalPhase::Ended => "ended exited",
        TerminalPhase::Lost => "lost",
        TerminalPhase::Archived => "archived",
        TerminalPhase::Finalizing => "finalizing finishing",
        TerminalPhase::OutputBlocked => "output blocked",
        TerminalPhase::Unreachable => "unreachable",
        TerminalPhase::Starting => "starting",
        TerminalPhase::Running => "running",
        TerminalPhase::Unavailable => "unknown unavailable not running",
    }
}

fn normalized(filter: &str) -> String {
    filter.trim().to_lowercase()
}

fn terminal_is_dead(phase: TerminalPhase) -> bool {
    matches!(
        phase,
        TerminalPhase::Ended | TerminalPhase::Lost | TerminalPhase::Archived
    )
}

fn agent_is_active(
    fields: &yolo_hub_agent_protocol::AgentSnapshotFields,
    session: Option<&yolo_hub_terminal_protocol::TerminalSummaryFields>,
    phase: Option<TerminalPhase>,
) -> bool {
    if matches!(fields.lifecycle, Lifecycle::Exited { .. })
        || fields.freshness != Freshness::Current
        || fields.reachability != Reachability::Reachable
    {
        return false;
    }
    match (session, phase) {
        (Some(session), Some(phase)) => {
            !terminal_is_dead(phase)
                && session.agent == Some(fields.cursor.target())
                && session
                    .runtime
                    .is_some_and(|runtime| runtime.target == fields.terminal)
        }
        // An active row must be actionable. A missing terminal observation has
        // no runtime target to open, so keep it available only in history.
        _ => false,
    }
}

fn matches(filter: &str, fields: &[&str]) -> bool {
    fields
        .iter()
        .any(|field| field.to_lowercase().contains(filter))
}
