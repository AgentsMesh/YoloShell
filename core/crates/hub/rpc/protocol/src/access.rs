use yolo_hub_routing::{Right, Rights};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Access {
    LocalManagement,
    Observe,
    TerminalRead,
    TerminalWrite,
    Lifecycle,
    FileRead,
    PresentationRead,
    PresentationWrite,
    ConversationRead,
    AgentControl,
    TerminalRun,
    AgentStart,
    AgentLifecycle,
    AgentMetadata,
    WorkspaceRead,
    WorkspaceWrite,
}

impl Access {
    /// Public resource rights shared by peer delegation and client admission.
    /// None denotes a local management operation, never an empty remote grant.
    pub fn required_rights(self) -> Option<Rights> {
        Some(match self {
            Self::LocalManagement => return None,
            Self::Observe => [Right::Observe].into_iter().collect(),
            Self::TerminalRead => [Right::TerminalRead].into_iter().collect(),
            Self::TerminalWrite => [Right::TerminalWrite].into_iter().collect(),
            Self::Lifecycle => [Right::Lifecycle].into_iter().collect(),
            Self::FileRead => [Right::FileRead].into_iter().collect(),
            Self::PresentationRead => [Right::PresentationRead].into_iter().collect(),
            Self::PresentationWrite => [Right::PresentationWrite].into_iter().collect(),
            Self::ConversationRead => [Right::ConversationRead].into_iter().collect(),
            Self::AgentControl => [Right::AgentControl, Right::TerminalWrite]
                .into_iter()
                .collect(),
            Self::TerminalRun => [Right::Lifecycle, Right::TerminalWrite]
                .into_iter()
                .collect(),
            Self::AgentStart => [Right::AgentControl, Right::Lifecycle, Right::TerminalWrite]
                .into_iter()
                .collect(),
            Self::AgentLifecycle => [Right::AgentControl, Right::Lifecycle]
                .into_iter()
                .collect(),
            Self::AgentMetadata => [Right::AgentControl].into_iter().collect(),
            Self::WorkspaceRead => [Right::WorkspaceRead].into_iter().collect(),
            Self::WorkspaceWrite => [Right::WorkspaceWrite].into_iter().collect(),
        })
    }
}
