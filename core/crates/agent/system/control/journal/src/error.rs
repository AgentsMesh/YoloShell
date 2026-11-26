use crate::constants::*;
use std::{error::Error, fmt};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JournalError {
    InvalidRequest,
    ScopeMismatch,
    Conflict,
    Capacity,
    ForeignReservation,
    AlreadyExists,
    Busy,
    Unavailable,
    OutcomeUnknown,
    Poisoned,
}
impl JournalError {
    pub(crate) fn poisons(self) -> bool {
        matches!(
            self,
            Self::Unavailable | Self::OutcomeUnknown | Self::Poisoned
        )
    }
}
impl fmt::Display for JournalError {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        output.write_str(match self {
            Self::InvalidRequest => INVALID_REQUEST,
            Self::ScopeMismatch => WRONG_ANCHOR,
            Self::Conflict => CONFLICT,
            Self::Capacity => CAPACITY,
            Self::ForeignReservation => FOREIGN_RESERVATION,
            Self::AlreadyExists => ALREADY_EXISTS,
            Self::Busy => BUSY,
            Self::Unavailable => UNAVAILABLE,
            Self::OutcomeUnknown => OUTCOME_UNKNOWN,
            Self::Poisoned => POISONED,
        })
    }
}
// Lower-level decoding errors can include record contents; do not expose their chains.
impl Error for JournalError {}
