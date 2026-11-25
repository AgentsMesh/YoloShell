use crate::event::{NotificationKind, SessionEndReason};
use crate::state::{AttentionState, TerminationCandidate};

pub(crate) fn termination_candidate(reason: &SessionEndReason) -> TerminationCandidate {
    match reason {
        SessionEndReason::Other => TerminationCandidate::Final,
        SessionEndReason::Clear => TerminationCandidate::Clear,
        SessionEndReason::Resume => TerminationCandidate::Resume,
    }
}

pub(crate) fn notification_attention(kind: &NotificationKind) -> AttentionState {
    match kind {
        NotificationKind::PermissionPrompt => AttentionState::Permission,
        NotificationKind::AgentNeedsInput => AttentionState::UserInput,
        NotificationKind::Elicitation => AttentionState::Elicitation,
        NotificationKind::Other(_) => AttentionState::None,
    }
}
