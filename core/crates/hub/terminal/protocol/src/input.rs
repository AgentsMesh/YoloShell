use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{LeaseGrant, MAX_INPUT_BYTES, PublicError, TerminalSize, validation};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "InputFields", into = "InputFields")]
pub struct InputRequest(InputFields);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputFields {
    /// A delivery identity, not permission to replay an unknown outcome.
    #[serde(
        serialize_with = "validation::serialize_id",
        deserialize_with = "validation::deserialize_id"
    )]
    pub request_id: Uuid,
    pub grant: LeaseGrant,
    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,
}

impl InputRequest {
    pub fn new(fields: InputFields) -> Result<Self, PublicError> {
        fields.try_into()
    }

    pub fn fields(&self) -> &InputFields {
        &self.0
    }
}

impl TryFrom<InputFields> for InputRequest {
    type Error = PublicError;

    fn try_from(fields: InputFields) -> Result<Self, Self::Error> {
        validation::request_id(fields.request_id)?;
        if fields.data.is_empty() || fields.data.len() > MAX_INPUT_BYTES {
            return Err(PublicError::InvalidPayload);
        }
        Ok(Self(fields))
    }
}

impl From<InputRequest> for InputFields {
    fn from(value: InputRequest) -> Self {
        value.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResizeRequest {
    #[serde(
        serialize_with = "validation::serialize_id",
        deserialize_with = "validation::deserialize_id"
    )]
    pub request_id: Uuid,
    pub grant: LeaseGrant,
    pub size: TerminalSize,
}

impl ResizeRequest {
    pub fn validate(self) -> Result<(), PublicError> {
        validation::request_id(self.request_id)
    }
}
