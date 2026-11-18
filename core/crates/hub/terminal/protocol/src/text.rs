//! Non-authoritative terminal text reads. The Worker owns the visible screen;
//! the text is presentation data and never lifecycle or control authority.

use serde::{Deserialize, Serialize};
use yolo_hub_identity::RuntimeTarget;
use yolo_terminal_metadata_protocol::MAX_TERMINAL_TEXT_BYTES;

use crate::{PublicError, TERMINAL_PROTOCOL_SCHEMA_VERSION, TerminalSize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadTerminalTextFields {
    pub schema_version: u16,
    pub target: RuntimeTarget,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "ReadTerminalTextFields", into = "ReadTerminalTextFields")]
pub struct ReadTerminalTextRequest(ReadTerminalTextFields);

impl ReadTerminalTextRequest {
    pub fn new(fields: ReadTerminalTextFields) -> Result<Self, PublicError> {
        fields.try_into()
    }

    pub fn fields(&self) -> &ReadTerminalTextFields {
        &self.0
    }
}

impl TryFrom<ReadTerminalTextFields> for ReadTerminalTextRequest {
    type Error = PublicError;

    fn try_from(fields: ReadTerminalTextFields) -> Result<Self, PublicError> {
        if fields.schema_version != TERMINAL_PROTOCOL_SCHEMA_VERSION {
            return Err(PublicError::UnsupportedSchema);
        }
        Ok(Self(fields))
    }
}

impl From<ReadTerminalTextRequest> for ReadTerminalTextFields {
    fn from(value: ReadTerminalTextRequest) -> Self {
        value.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerminalTextFields {
    pub target: RuntimeTarget,
    pub sequence: u64,
    pub size: TerminalSize,
    pub text: String,
    pub truncated: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "TerminalTextFields", into = "TerminalTextFields")]
pub struct TerminalText(TerminalTextFields);

impl TerminalText {
    pub fn new(fields: TerminalTextFields) -> Result<Self, PublicError> {
        fields.try_into()
    }

    pub fn fields(&self) -> &TerminalTextFields {
        &self.0
    }
}

impl TryFrom<TerminalTextFields> for TerminalText {
    type Error = PublicError;

    fn try_from(fields: TerminalTextFields) -> Result<Self, PublicError> {
        if fields.text.len() > MAX_TERMINAL_TEXT_BYTES {
            return Err(PublicError::InvalidPayload);
        }
        Ok(Self(fields))
    }
}

impl From<TerminalText> for TerminalTextFields {
    fn from(value: TerminalText) -> Self {
        value.0
    }
}
