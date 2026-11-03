use yolo_hub_agent_protocol::{AgentSnapshot, Freshness, Lifecycle, Reachability};
use yolo_hub_terminal_protocol::{RuntimeStatus, TerminalSummaryFields};

/// A historical provider session is not evidence of an active agent invocation.
pub fn active_agent<'a>(
    terminal: &TerminalSummaryFields,
    candidate: Option<&'a AgentSnapshot>,
    catalog_current: bool,
) -> Option<&'a AgentSnapshot> {
    let runtime = terminal.runtime?;
    let agent = candidate?;
    let fields = agent.fields();
    (catalog_current
        && !terminal.archived
        && !matches!(
            runtime.status,
            RuntimeStatus::Exited | RuntimeStatus::Lost | RuntimeStatus::Unreachable
        )
        && terminal.agent == Some(fields.cursor.target())
        && runtime.target == fields.terminal
        && fields.freshness == Freshness::Current
        && fields.reachability == Reachability::Reachable
        && matches!(
            fields.lifecycle,
            Lifecycle::Starting {} | Lifecycle::Running {}
        ))
    .then_some(agent)
}
