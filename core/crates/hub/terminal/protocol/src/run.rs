use serde::{Deserialize, Serialize};
use uuid::Uuid;
use yolo_hub_identity::TerminalAddress;

use crate::{
    MAX_DIRECTORY_BYTES, MAX_INPUT_BYTES, MAX_TERMINAL_NAME_BYTES, PublicError,
    TERMINAL_PROTOCOL_SCHEMA_VERSION, TerminalSize, validation,
};

/// Create a shell and write one bounded input burst after the Worker's hook
/// readiness gate. The Hub finalizes the creation before returning, so the
/// caller receives a usable terminal rather than a creation capability.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "TerminalRunFields", into = "TerminalRunFields")]
pub struct TerminalRunRequest(TerminalRunFields);

#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerminalRunFields {
    pub schema_version: u16,
    #[serde(
        serialize_with = "validation::serialize_id",
        deserialize_with = "validation::deserialize_id"
    )]
    pub request_id: Uuid,
    #[serde(
        serialize_with = "validation::serialize_id",
        deserialize_with = "validation::deserialize_id"
    )]
    pub creation_token: Uuid,
    pub name: Option<String>,
    pub cwd: String,
    pub size: TerminalSize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<Uuid>,
    #[serde(with = "serde_bytes")]
    pub input: Vec<u8>,
}

impl std::fmt::Debug for TerminalRunFields {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("TerminalRunFields")
            .field(crate::field::SCHEMA_VERSION, &self.schema_version)
            .field(crate::field::REQUEST_ID, &self.request_id)
            .field(crate::field::CREATION_TOKEN, &self.creation_token)
            .field(crate::field::NAME, &self.name)
            .field(crate::field::CWD, &self.cwd)
            .field(crate::field::COLS, &self.size.cols())
            .field(crate::field::ROWS, &self.size.rows())
            .field(crate::field::DATA, &crate::constants::REDACTED_CAPABILITY)
            .field(yolo_hub_identity::field::WORKSPACE_ID, &self.workspace_id)
            .finish()
    }
}

impl TerminalRunRequest {
    pub fn new(fields: TerminalRunFields) -> Result<Self, PublicError> {
        fields.try_into()
    }

    pub fn fields(&self) -> &TerminalRunFields {
        &self.0
    }
}

impl TryFrom<TerminalRunFields> for TerminalRunRequest {
    type Error = PublicError;

    fn try_from(fields: TerminalRunFields) -> Result<Self, Self::Error> {
        if fields.schema_version != TERMINAL_PROTOCOL_SCHEMA_VERSION {
            return Err(PublicError::UnsupportedSchema);
        }
        validation::request_id(fields.request_id)?;
        validation::request_id(fields.creation_token)?;
        if !validation::metadata(&fields.cwd, MAX_DIRECTORY_BYTES)
            || fields
                .name
                .as_ref()
                .is_some_and(|name| !validation::metadata(name, MAX_TERMINAL_NAME_BYTES))
        {
            return Err(PublicError::InvalidMetadata);
        }
        if fields
            .workspace_id
            .is_some_and(|workspace| workspace.is_nil())
        {
            return Err(PublicError::InvalidRequest);
        }
        if fields.input.is_empty() || fields.input.len() > MAX_INPUT_BYTES {
            return Err(PublicError::InvalidPayload);
        }
        Ok(Self(fields))
    }
}

impl From<TerminalRunRequest> for TerminalRunFields {
    fn from(value: TerminalRunRequest) -> Self {
        value.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerminalRunResult {
    pub address: TerminalAddress,
}
