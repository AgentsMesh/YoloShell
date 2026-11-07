use crate::Command;
use contract::{HUB_CREATE_TIMEOUT, HUB_LIFECYCLE_TIMEOUT, HUB_RESUME_TIMEOUT};
use std::time::Duration;
use yolo_hub_terminal_protocol::TerminalAction;

mod contract {
    include!("generated/hub_lifecycle_deadline_contract.rs");
}

impl Command {
    /// Preserve the Host lifecycle budget plus the existing forwarding response
    /// margin. Ordinary RPCs retain their caller's configured deadline.
    /// This is a per-operation floor, not a propagated end-to-end route deadline.
    pub fn operation_timeout(&self, configured: Duration) -> Duration {
        let floor = match self {
            Self::Create { .. } | Self::TerminalRun { .. } | Self::AgentStart { .. } => {
                HUB_CREATE_TIMEOUT
            }
            Self::Terminal { command } => match command.action {
                TerminalAction::Resume {} | TerminalAction::Recreate {} => HUB_RESUME_TIMEOUT,
                TerminalAction::Terminate { .. } | TerminalAction::Archive {} => {
                    HUB_LIFECYCLE_TIMEOUT
                }
            },
            Self::AgentStop { .. }
            | Self::Finalize { .. }
            | Self::CleanupTerminate { .. }
            | Self::CleanupArchive { .. } => HUB_LIFECYCLE_TIMEOUT,
            _ => return configured,
        };
        configured.max(floor)
    }
}
