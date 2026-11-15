use crate::constants;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProtocolError {
    InvalidId,
    InvalidTitle,
    InvalidTree,
    Limit,
    DuplicateId,
    InvalidRevision,
    RevisionConflict,
    AddressMismatch,
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidId => constants::INVALID_ID,
            Self::InvalidTitle => constants::INVALID_TITLE,
            Self::InvalidTree => constants::INVALID_TREE,
            Self::Limit => constants::LIMIT,
            Self::DuplicateId => constants::DUPLICATE,
            Self::InvalidRevision => constants::REVISION,
            Self::RevisionConflict => constants::CONFLICT,
            Self::AddressMismatch => constants::ADDRESS,
        })
    }
}
impl std::error::Error for ProtocolError {}
