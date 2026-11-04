use serde::{Deserialize, Serialize};
use uuid::Uuid;
use yolo_hub_identity::TerminalAddress;

use crate::{
    MAX_DIRECTORY_BYTES, MAX_TERMINAL_NAME_BYTES, PublicError, TERMINAL_PROTOCOL_SCHEMA_VERSION,
    SecretCapability, TerminalSize, validation,
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "CreateShellFields", into = "CreateShellFields")]
pub struct CreateShellRequest(CreateShellFields);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateShellFields {
    pub schema_version: u16,
    #[serde(
        serialize_with = "validation::serialize_id",
        deserialize_with = "validation::deserialize_id"
    )]
    pub request_id: Uuid,
    /// Stable across retries and reconnects, separate from this RPC's correlation ID.
    #[serde(
        serialize_with = "validation::serialize_id",
        deserialize_with = "validation::deserialize_id"
    )]
    pub creation_token: Uuid,
    pub name: Option<String>,
    pub cwd: String,
    pub size: TerminalSize,
}

impl CreateShellRequest {
    pub fn new(fields: CreateShellFields) -> Result<Self, PublicError> {
        fields.try_into()
    }

    pub fn fields(&self) -> &CreateShellFields {
        &self.0
    }
}

impl TryFrom<CreateShellFields> for CreateShellRequest {
    type Error = PublicError;

    fn try_from(fields: CreateShellFields) -> Result<Self, Self::Error> {
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
        Ok(Self(fields))
    }
}

impl From<CreateShellRequest> for CreateShellFields {
    fn from(value: CreateShellRequest) -> Self {
        value.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateShellResult {
    pub address: TerminalAddress,
    pub creation_capability: SecretCapability,
}

/// The origin preserves a survivor when it cannot prove safe creation cleanup.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum CleanupTerminationOutcome {
    Terminated,
    Survivor,
}
