use crate::constants::*;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectorError {
    UnsupportedSchema,
    InvalidRequest,
    Unauthorized,
    NotFound,
    Conflict,
    StaleGeneration,
    Sequence,
    Unavailable,
    Capacity,
    OutcomeUnknown,
    Internal,
}

impl fmt::Display for ConnectorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::UnsupportedSchema => UNSUPPORTED_SCHEMA_ERROR,
            Self::InvalidRequest => INVALID_REQUEST_ERROR,
            Self::Unauthorized => UNAUTHORIZED_ERROR,
            Self::NotFound => NOT_FOUND_ERROR,
            Self::Conflict => CONFLICT_ERROR,
            Self::StaleGeneration => STALE_GENERATION_ERROR,
            Self::Sequence => SEQUENCE_ERROR,
            Self::Unavailable => UNAVAILABLE_ERROR,
            Self::Capacity => CAPACITY_ERROR,
            Self::OutcomeUnknown => OUTCOME_UNKNOWN_ERROR,
            Self::Internal => INTERNAL_ERROR,
        })
    }
}

impl std::error::Error for ConnectorError {}
