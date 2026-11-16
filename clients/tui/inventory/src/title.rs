use std::collections::BTreeMap;
use yolo_hub_agent_protocol::AgentSnapshot;
use yolo_hub_identity::TerminalAddress;
use yolo_hub_terminal_protocol::TerminalSummary;

/// A reused Terminal must never supply the title of a different execution.
pub fn agent_title<'a>(
    agent: &'a AgentSnapshot,
    terminals: &'a BTreeMap<TerminalAddress, TerminalSummary>,
) -> &'a str {
    let fields = agent.fields();
    terminals
        .get(&fields.terminal.address)
        .filter(|terminal| {
            terminal
                .fields()
                .runtime
                .is_some_and(|runtime| runtime.target == fields.terminal)
                && terminal.fields().agent == Some(fields.cursor.target())
        })
        .and_then(|terminal| terminal.fields().presentation.title.as_deref())
        .filter(|title| !title.is_empty())
        .unwrap_or_else(|| {
            fields
                .provider
                .display_name()
                .unwrap_or_else(|| fields.provider.kind())
        })
}
