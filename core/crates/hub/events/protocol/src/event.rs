//! Causally recorded platform events and their stream identity.

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use yolo_hub_agent_protocol::AgentStopOutcome;
use yolo_hub_agent_run_protocol::{AgentRunAddress, AgentRunState};
use yolo_hub_identity::TerminalAddress;
use yolo_hub_workspace_protocol::WorkspaceAddress;
use yolo_presentation_protocol::LayoutAddress;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformEventCursor {
    pub epoch: Uuid,
    pub sequence: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformEventStream {
    /// Per-Hub-process random epoch. A changed epoch means the retained event
    /// buffer restarted and clients must reconcile with a snapshot.
    pub epoch: Uuid,
    /// Latest assigned sequence; `0` means no event has been recorded.
    pub sequence: u64,
    /// Oldest retained sequence; `sequence + 1` when the buffer is empty.
    pub first_sequence: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PlatformEventKind {
    TerminalCreated {
        address: TerminalAddress,
    },
    TerminalChanged {
        address: TerminalAddress,
        archived: bool,
    },
    AgentStarted {
        terminal: TerminalAddress,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        run: Option<AgentRunAddress>,
    },
    AgentStopped {
        run: AgentRunAddress,
        outcome: AgentStopOutcome,
    },
    RunUpdated {
        address: AgentRunAddress,
        revision: u64,
        state: AgentRunState,
    },
    WorkspaceChanged {
        address: WorkspaceAddress,
        revision: u64,
    },
    /// A shared layout committed a new topology revision. Client focus,
    /// scrolling and writer grants are not part of this event.
    LayoutChanged {
        address: LayoutAddress,
        revision: u64,
    },
}

impl PlatformEventKind {
    /// Origin Hub of the affected resource, if it names one.
    pub fn hub_id(&self) -> Option<yolo_hub_identity::HubId> {
        match self {
            Self::TerminalCreated { address } | Self::TerminalChanged { address, .. } => {
                Some(address.hub_id)
            }
            Self::AgentStarted { terminal, .. } => Some(terminal.hub_id),
            Self::AgentStopped { run, .. } | Self::RunUpdated { address: run, .. } => {
                Some(run.hub_id)
            }
            Self::WorkspaceChanged { address, .. } => Some(address.hub_id),
            Self::LayoutChanged { address, .. } => Some(address.hub_id),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformEvent {
    pub sequence: u64,
    pub at_ms: i64,
    pub kind: PlatformEventKind,
}
