use crate::constants;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LayoutError {
    UnknownPane,
    LastPane,
    InvalidId,
    InvalidRatio,
    Limit,
    InvalidRect,
    InvalidSplitPath,
}

impl fmt::Display for LayoutError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::UnknownPane => constants::UNKNOWN_PANE,
            Self::LastPane => constants::LAST_PANE,
            Self::InvalidId => constants::INVALID_ID,
            Self::InvalidRatio => constants::INVALID_RATIO,
            Self::Limit => constants::LAYOUT_LIMIT,
            Self::InvalidRect => constants::INVALID_RECT,
            Self::InvalidSplitPath => constants::INVALID_SPLIT_PATH,
        })
    }
}

impl std::error::Error for LayoutError {}
