//! Bounded, read-only history of an archived terminal. The Hub returns the
//! retained raw output stream; clients replay it into their terminal emulator.

use base64::Engine;
use serde::{Deserialize, Serialize};
use yolo_hub_identity::RuntimeTarget;

use crate::{PublicError, TERMINAL_PROTOCOL_SCHEMA_VERSION};

pub const MAX_TERMINAL_HISTORY_BYTES: usize = 4 * 1024 * 1024;
pub const DEFAULT_TERMINAL_HISTORY_BYTES: u32 = 2 * 1024 * 1024;

const MAX_ENCODED_BYTES: usize = MAX_TERMINAL_HISTORY_BYTES * 4 / 3 + 8;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadTerminalHistoryFields {
    pub schema_version: u16,
    pub target: RuntimeTarget,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_bytes: Option<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "ReadTerminalHistoryFields", into = "ReadTerminalHistoryFields")]
pub struct ReadTerminalHistoryRequest(ReadTerminalHistoryFields);

impl ReadTerminalHistoryRequest {
    pub fn new(fields: ReadTerminalHistoryFields) -> Result<Self, PublicError> {
        fields.try_into()
    }

    pub fn fields(&self) -> &ReadTerminalHistoryFields {
        &self.0
    }

    pub fn max_bytes(&self) -> u32 {
        self.0.max_bytes.unwrap_or(DEFAULT_TERMINAL_HISTORY_BYTES)
    }
}

impl TryFrom<ReadTerminalHistoryFields> for ReadTerminalHistoryRequest {
    type Error = PublicError;

    fn try_from(fields: ReadTerminalHistoryFields) -> Result<Self, PublicError> {
        if fields.schema_version != TERMINAL_PROTOCOL_SCHEMA_VERSION {
            return Err(PublicError::UnsupportedSchema);
        }
        if fields
            .max_bytes
            .is_some_and(|value| value == 0 || value as usize > MAX_TERMINAL_HISTORY_BYTES)
        {
            return Err(PublicError::InvalidPayload);
        }
        Ok(Self(fields))
    }
}

impl From<ReadTerminalHistoryRequest> for ReadTerminalHistoryFields {
    fn from(value: ReadTerminalHistoryRequest) -> Self {
        value.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerminalHistoryReadFields {
    pub target: RuntimeTarget,
    /// Standard base64 of the retained output stream.
    pub data: String,
    pub from_sequence: u64,
    pub next_sequence: u64,
    pub truncated: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "TerminalHistoryReadFields", into = "TerminalHistoryReadFields")]
pub struct TerminalHistoryRead(TerminalHistoryReadFields);

impl TerminalHistoryRead {
    pub fn new(fields: TerminalHistoryReadFields) -> Result<Self, PublicError> {
        fields.try_into()
    }

    pub fn fields(&self) -> &TerminalHistoryReadFields {
        &self.0
    }

    /// The raw terminal stream after validating its encoded and decoded bounds.
    pub fn bytes(&self) -> Result<Vec<u8>, PublicError> {
        base64::engine::general_purpose::STANDARD
            .decode(&self.0.data)
            .map_err(|_| PublicError::InvalidPayload)
    }
}

impl TryFrom<TerminalHistoryReadFields> for TerminalHistoryRead {
    type Error = PublicError;

    fn try_from(fields: TerminalHistoryReadFields) -> Result<Self, PublicError> {
        if fields.data.len() > MAX_ENCODED_BYTES
            || fields.next_sequence == 0
            || fields.from_sequence > fields.next_sequence
        {
            return Err(PublicError::InvalidPayload);
        }
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(&fields.data)
            .map_err(|_| PublicError::InvalidPayload)?;
        if decoded.len() > MAX_TERMINAL_HISTORY_BYTES {
            return Err(PublicError::InvalidPayload);
        }
        Ok(Self(fields))
    }
}

impl From<TerminalHistoryRead> for TerminalHistoryReadFields {
    fn from(value: TerminalHistoryRead) -> Self {
        value.0
    }
}
