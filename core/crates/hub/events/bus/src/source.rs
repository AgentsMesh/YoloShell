//! Mapping from a confirmed public effect to its platform events.

use yolo_hub_events_protocol::PlatformEventKind;
use yolo_hub_rpc_protocol::{Command, Reply};

/// A removed resource has no successor revision; clients refresh by address.
const REMOVED_WORKSPACE_REVISION: u64 = 0;

pub(crate) fn kinds_for(command: &Command, reply: &Reply) -> Vec<PlatformEventKind> {
    let kind = match (command, reply) {
        (Command::Create { .. }, Reply::Created { creation }) => {
            PlatformEventKind::TerminalCreated {
                address: creation.address,
            }
        }
        (Command::TerminalRun { .. }, Reply::TerminalRun { run }) => {
            PlatformEventKind::TerminalCreated {
                address: run.address,
            }
        }
        (
            Command::Terminal { .. } | Command::CleanupArchive { .. },
            Reply::Terminal { terminal },
        ) => PlatformEventKind::TerminalChanged {
            address: terminal.fields().address,
            archived: terminal.fields().archived,
        },
        (Command::CleanupTerminate { .. }, Reply::CreationTerminated { address, .. }) => {
            PlatformEventKind::TerminalChanged {
                address: *address,
                archived: false,
            }
        }
        (Command::AgentStart { .. }, Reply::AgentStart { start }) => {
            PlatformEventKind::AgentStarted {
                terminal: start.address,
                run: start.run.as_ref().map(|run| run.address),
            }
        }
        (Command::AgentStop { .. }, Reply::AgentStop { stop }) => PlatformEventKind::AgentStopped {
            run: stop.run.address,
            outcome: stop.outcome,
        },
        (Command::UpdateAgentRun { .. }, Reply::AgentRun { run }) => {
            PlatformEventKind::RunUpdated {
                address: run.address,
                revision: run.revision,
                state: run.state,
            }
        }
        (
            Command::CreateWorkspace { .. }
            | Command::UpdateWorkspace { .. }
            | Command::CreateWorktree { .. },
            Reply::Workspace { workspace },
        ) => PlatformEventKind::WorkspaceChanged {
            address: workspace.address,
            revision: workspace.revision,
        },
        (Command::RemoveWorktree { change }, Reply::Completed {}) => {
            PlatformEventKind::WorkspaceChanged {
                address: change.fields().address,
                revision: REMOVED_WORKSPACE_REVISION,
            }
        }
        (
            Command::UpdateLayout { change },
            Reply::Layout {
                document: Some(document),
            },
        ) => PlatformEventKind::LayoutChanged {
            address: change.address,
            revision: document.revision,
        },
        _ => return Vec::new(),
    };
    vec![kind]
}
