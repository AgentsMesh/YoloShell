use yolo_hub_events_protocol::PlatformEventKind;

pub(crate) fn label(kind: &PlatformEventKind) -> String {
    match kind {
        PlatformEventKind::TerminalCreated { address } => {
            format!("terminal_created {}", address.terminal_id)
        }
        PlatformEventKind::TerminalChanged { address, archived } => {
            format!(
                "terminal_changed {} archived={archived}",
                address.terminal_id
            )
        }
        PlatformEventKind::AgentStarted { terminal, .. } => {
            format!("agent_started {}", terminal.terminal_id)
        }
        PlatformEventKind::AgentStopped { run, outcome } => {
            format!("agent_stopped {} {outcome:?}", run.run_id)
        }
        PlatformEventKind::RunUpdated {
            address, revision, ..
        } => {
            format!("run_updated {} revision={revision}", address.run_id)
        }
        PlatformEventKind::WorkspaceChanged { address, revision } => {
            format!(
                "workspace_changed {} revision={revision}",
                address.workspace_id
            )
        }
        PlatformEventKind::LayoutChanged { address, revision } => {
            format!("layout_changed {} revision={revision}", address.layout_id)
        }
    }
}
