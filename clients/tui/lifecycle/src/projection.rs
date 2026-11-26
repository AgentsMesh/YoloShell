use crate::{HUB_RECONNECTING_LABEL, TERMINAL_RECONNECTING_LABEL, TerminalPhase};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConnectionState {
    Connected,
    HubReconnecting,
    TerminalReconnecting,
}

impl ConnectionState {
    pub fn label(self) -> Option<&'static str> {
        match self {
            Self::Connected => None,
            Self::HubReconnecting => Some(HUB_RECONNECTING_LABEL),
            Self::TerminalReconnecting => Some(TERMINAL_RECONNECTING_LABEL),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputAccess {
    Unavailable,
    ReadOnly,
    Synchronizing,
    Writing,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TerminalProjection {
    pub phase: TerminalPhase,
    pub connection: ConnectionState,
    pub input: InputAccess,
}

impl TerminalProjection {
    pub fn label(self) -> &'static str {
        if self.phase != TerminalPhase::Running {
            self.phase.label()
        } else if let Some(label) = self.connection.label() {
            label
        } else {
            match self.input {
                InputAccess::Writing => crate::WRITING_LABEL,
                InputAccess::Synchronizing => crate::SYNCING_LABEL,
                InputAccess::ReadOnly | InputAccess::Unavailable => crate::READ_ONLY_LABEL,
            }
        }
    }

    pub fn action_hint(self) -> Option<&'static str> {
        match self.phase {
            TerminalPhase::Ended | TerminalPhase::Archived => Some(crate::ENDED_ACTION),
            TerminalPhase::Lost => Some(crate::LOST_HINT),
            TerminalPhase::OutputBlocked => Some(crate::OUTPUT_BLOCKED_ACTION),
            _ => None,
        }
    }

    pub fn is_warning(self) -> bool {
        self.connection != ConnectionState::Connected
            || matches!(
                self.phase,
                TerminalPhase::OutputBlocked | TerminalPhase::Lost | TerminalPhase::Unreachable
            )
            || self.input == InputAccess::ReadOnly
    }

    pub fn new(
        phase: TerminalPhase,
        connection: ConnectionState,
        has_writer: bool,
        presentation_ready: bool,
    ) -> Self {
        let input = if !phase.allows_input() || connection != ConnectionState::Connected {
            InputAccess::Unavailable
        } else if !has_writer {
            InputAccess::ReadOnly
        } else if !presentation_ready {
            InputAccess::Synchronizing
        } else {
            InputAccess::Writing
        };
        Self {
            phase,
            connection,
            input,
        }
    }
}
