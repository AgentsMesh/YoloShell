use crate::{
    CreateShellRequest, CreateShellResult, PublicError, constants::REDACTED_CAPABILITY, validation,
};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;
use yolo_hub_identity::{HubId, TerminalAddress};

const REQUEST_DEBUG_NAME: &str = "CreationQueryRequest";
const RESULT_DEBUG_NAME: &str = "CreationQueryResult";
const OUTCOME_DEBUG_FIELD: &str = "outcome";

/// Original creation identity and parameters, used only to inspect durable
/// origin evidence. Constructing or sending this request never replays Create.
#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CreationQueryRequest(CreateShellRequest);

impl CreationQueryRequest {
    pub fn new(original: CreateShellRequest) -> Self {
        Self(original)
    }
    pub fn creation(&self) -> &CreateShellRequest {
        &self.0
    }
}

impl fmt::Debug for CreationQueryRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple(REQUEST_DEBUG_NAME)
            .field(&REDACTED_CAPABILITY)
            .finish()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum CreationQueryOutcome {
    /// No matching evidence in this read. A delayed Create may still arrive.
    Missing {},
    /// Creation has an origin reservation but no confirmed ready result.
    Pending {},
    /// Durable evidence cannot establish an exact result, including old receipts.
    Unknown {},
    Ready {
        creation: CreateShellResult,
    },
    Finalized {
        creation: CreateShellResult,
    },
    Survivor {
        creation: CreateShellResult,
    },
    Archived {
        address: TerminalAddress,
    },
}

impl CreationQueryOutcome {
    fn address(&self) -> Option<TerminalAddress> {
        match self {
            Self::Ready { creation }
            | Self::Finalized { creation }
            | Self::Survivor { creation } => Some(creation.address),
            Self::Archived { address } => Some(*address),
            Self::Missing {} | Self::Pending {} | Self::Unknown {} => None,
        }
    }
}

#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "CreationQueryFields", into = "CreationQueryFields")]
pub struct CreationQueryResult(CreationQueryFields);

#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CreationQueryFields {
    #[serde(
        serialize_with = "validation::serialize_id",
        deserialize_with = "validation::deserialize_id"
    )]
    creation_token: Uuid,
    outcome: CreationQueryOutcome,
}

impl CreationQueryResult {
    pub fn new(creation_token: Uuid, outcome: CreationQueryOutcome) -> Result<Self, PublicError> {
        CreationQueryFields {
            creation_token,
            outcome,
        }
        .try_into()
    }
    pub fn creation_token(&self) -> Uuid {
        self.0.creation_token
    }
    pub fn outcome(&self) -> &CreationQueryOutcome {
        &self.0.outcome
    }

    pub fn validate_for(&self, token: Uuid, origin: HubId) -> Result<(), PublicError> {
        validation::request_id(token)?;
        if self.creation_token() != token {
            return Err(PublicError::InvalidRequest);
        }
        if self
            .outcome()
            .address()
            .is_some_and(|address| address.hub_id != origin)
        {
            return Err(PublicError::OriginMismatch);
        }
        Ok(())
    }
}

impl TryFrom<CreationQueryFields> for CreationQueryResult {
    type Error = PublicError;
    fn try_from(fields: CreationQueryFields) -> Result<Self, Self::Error> {
        validation::request_id(fields.creation_token)?;
        Ok(Self(fields))
    }
}

impl From<CreationQueryResult> for CreationQueryFields {
    fn from(value: CreationQueryResult) -> Self {
        value.0
    }
}

impl fmt::Debug for CreationQueryResult {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct(RESULT_DEBUG_NAME)
            .field(crate::field::CREATION_TOKEN, &REDACTED_CAPABILITY)
            .field(OUTCOME_DEBUG_FIELD, self.outcome())
            .finish()
    }
}
