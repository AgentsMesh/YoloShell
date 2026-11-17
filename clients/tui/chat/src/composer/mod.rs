mod editing;
mod submission;
use yolo_hub_agent_protocol::{AgentControlRequest, CommandReceipt, FailureCode};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DraftError {
    Locked,
    TooLarge,
    UnsafeText,
    Empty,
    WrongTarget,
    Unavailable,
    ObservationPending,
}
impl std::fmt::Display for DraftError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use crate::constants::*;
        formatter.write_str(match self {
            Self::Locked => DRAFT_LOCKED,
            Self::TooLarge => DRAFT_LARGE,
            Self::UnsafeText => DRAFT_UNSAFE,
            Self::Empty => DRAFT_EMPTY,
            Self::WrongTarget => DRAFT_TARGET,
            Self::Unavailable => DRAFT_UNAVAILABLE,
            Self::ObservationPending => INPUT_REFRESHING,
        })
    }
}
impl std::error::Error for DraftError {}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SendPhase {
    Sending,
    Accepted,
    Delivered,
    Confirmed,
    Unknown,
    NotSent,
    Rejected(FailureCode),
}
impl SendPhase {
    pub fn blocks(self) -> bool {
        matches!(self, Self::Sending | Self::Accepted | Self::Unknown)
    }
    pub fn label(self) -> &'static str {
        use crate::constants::*;
        match self {
            Self::Sending => SENDING,
            Self::Accepted => ACCEPTED,
            Self::Delivered => DELIVERED,
            Self::Confirmed => CONFIRMED,
            Self::Unknown => UNKNOWN,
            Self::NotSent => NOT_SENT,
            Self::Rejected(_) => REJECTED,
        }
    }
}

pub(crate) struct Submission {
    pub request: AgentControlRequest,
    pub receipt: CommandReceipt,
    pub phase: SendPhase,
    pub awaiting_dispatch_result: bool,
}

#[derive(Default)]
pub struct Composer {
    pub(crate) text: String,
    cursor: usize,
    pub(crate) submission: Option<Submission>,
}
impl Composer {
    pub fn text(&self) -> &str {
        &self.text
    }
    pub fn cursor(&self) -> usize {
        self.cursor
    }
    pub fn phase(&self) -> Option<SendPhase> {
        self.submission.as_ref().map(|item| item.phase)
    }
    pub fn request(&self) -> Option<&AgentControlRequest> {
        self.submission.as_ref().map(|item| &item.request)
    }
    pub fn locked(&self) -> bool {
        self.phase().is_some_and(SendPhase::blocks)
    }
    fn editable(&self) -> Result<(), DraftError> {
        if self.locked() {
            Err(DraftError::Locked)
        } else {
            Ok(())
        }
    }
    pub fn clear(&mut self) -> Result<(), DraftError> {
        self.editable()?;
        self.text.clear();
        self.cursor = 0;
        Ok(())
    }
    /// Explicit acknowledgement after inspecting the terminal; it never resubmits input.
    pub fn discard_uncertain(&mut self) -> bool {
        if self.phase() != Some(SendPhase::Unknown) {
            return false;
        }
        self.submission = None;
        self.text.clear();
        self.cursor = 0;
        true
    }
}
