use super::SessionStartSource;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionEndReason {
    Other,
    Clear,
    Resume,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionEndBoundary {
    Recovering,
    FinalCandidate,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionEndTermination {
    pub reason: SessionEndReason,
    pub boundary: SessionEndBoundary,
}

impl SessionEndReason {
    pub(crate) fn boundary(self) -> SessionEndBoundary {
        match self {
            Self::Clear | Self::Resume => SessionEndBoundary::Recovering,
            Self::Other => SessionEndBoundary::FinalCandidate,
        }
    }
}

impl SessionEndBoundary {
    pub fn from_reason(reason: SessionEndReason) -> Self {
        reason.boundary()
    }
}

impl From<crate::claude_hooks::ClaudeSessionEndReason> for SessionEndReason {
    fn from(reason: crate::claude_hooks::ClaudeSessionEndReason) -> Self {
        match reason {
            crate::claude_hooks::ClaudeSessionEndReason::Other => Self::Other,
            crate::claude_hooks::ClaudeSessionEndReason::Clear => Self::Clear,
            crate::claude_hooks::ClaudeSessionEndReason::Resume => Self::Resume,
        }
    }
}

impl SessionEndTermination {
    pub fn from_reason(reason: SessionEndReason) -> Self {
        Self {
            reason,
            boundary: SessionEndBoundary::from_reason(reason),
        }
    }

    pub(crate) fn valid(self) -> bool {
        self.boundary == self.reason.boundary()
    }

    pub(crate) fn accepts_start(self, source: SessionStartSource) -> bool {
        self.boundary == SessionEndBoundary::Recovering
            && matches!(
                (self.reason, source),
                (SessionEndReason::Clear, SessionStartSource::Clear)
                    | (SessionEndReason::Resume, SessionStartSource::Resume)
            )
    }
}
