use serde::{Deserialize, Serialize};

use crate::{
    DEFAULT_TERMINAL_COLUMNS, DEFAULT_TERMINAL_ROWS, MAX_TERMINAL_COLUMNS, MAX_TERMINAL_ROWS,
    MIN_TERMINAL_COLUMNS, MIN_TERMINAL_ROWS, PublicError,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "SizeFields", into = "SizeFields")]
pub struct TerminalSize(SizeFields);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SizeFields {
    cols: u16,
    rows: u16,
}

impl TerminalSize {
    pub fn new(cols: u16, rows: u16) -> Result<Self, PublicError> {
        SizeFields { cols, rows }.try_into()
    }

    pub const fn cols(self) -> u16 {
        self.0.cols
    }

    pub const fn rows(self) -> u16 {
        self.0.rows
    }
}

impl Default for TerminalSize {
    fn default() -> Self {
        Self(SizeFields {
            cols: DEFAULT_TERMINAL_COLUMNS,
            rows: DEFAULT_TERMINAL_ROWS,
        })
    }
}

impl TryFrom<SizeFields> for TerminalSize {
    type Error = PublicError;

    fn try_from(fields: SizeFields) -> Result<Self, Self::Error> {
        if !(MIN_TERMINAL_COLUMNS..=MAX_TERMINAL_COLUMNS).contains(&fields.cols)
            || !(MIN_TERMINAL_ROWS..=MAX_TERMINAL_ROWS).contains(&fields.rows)
        {
            return Err(PublicError::InvalidSize);
        }
        Ok(Self(fields))
    }
}

impl From<TerminalSize> for SizeFields {
    fn from(value: TerminalSize) -> Self {
        value.0
    }
}
