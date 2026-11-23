use crate::constants::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectionError {
    EmptyViewport,
    CaptureLimit,
    InvalidPoint,
    CopyLimit,
    Incomplete,
    EmptySelection,
}

impl std::fmt::Display for SelectionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::EmptyViewport => EMPTY_VIEWPORT,
            Self::CaptureLimit => CAPTURE_LIMIT,
            Self::InvalidPoint => INVALID_POINT,
            Self::CopyLimit => COPY_LIMIT,
            Self::Incomplete => INCOMPLETE,
            Self::EmptySelection => EMPTY_SELECTION,
        })
    }
}

impl std::error::Error for SelectionError {}
