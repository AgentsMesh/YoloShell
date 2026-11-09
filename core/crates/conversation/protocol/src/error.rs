use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConversationError {
    InvalidRequest,
    BindingChanged,
    UnsupportedProvider,
    EvidenceUnavailable,
    EvidenceChanged,
    HistoryChanged,
    HistoryLimitExceeded,
    ReaderBusy,
}

impl fmt::Display for ConversationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidRequest => "invalid conversation request",
            Self::BindingChanged => "conversation binding changed",
            Self::UnsupportedProvider => "provider has no conversation adapter",
            Self::EvidenceUnavailable => "confirmed conversation evidence is unavailable",
            Self::EvidenceChanged => "conversation evidence changed while reading",
            Self::HistoryChanged => "conversation history changed; reload the current history",
            Self::HistoryLimitExceeded => "conversation exceeds the bounded history scan limit",
            Self::ReaderBusy => {
                "conversation reader capacity is busy; retry after an active read completes"
            }
        })
    }
}

impl std::error::Error for ConversationError {}
