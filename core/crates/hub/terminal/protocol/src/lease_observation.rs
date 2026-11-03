use serde::{Deserialize, Serialize};
use yolo_hub_identity::RuntimeTarget;

use crate::{FIRST_LEASE_GENERATION, MAX_WRITER_ID_BYTES, PublicError, validation::metadata};

/// Read-only holder metadata. A holder label is not an authenticated caller identity.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "LeaseObservationFields", into = "LeaseObservationFields")]
pub struct LeaseObservation(LeaseObservationFields);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LeaseObservationFields {
    pub target: RuntimeTarget,
    pub generation: u64,
    pub holder_id: String,
    pub controller_instance_id: Option<String>,
    pub expires_at_ms: i64,
}

impl LeaseObservation {
    pub fn new(fields: LeaseObservationFields) -> Result<Self, PublicError> {
        fields.try_into()
    }
    pub fn fields(&self) -> &LeaseObservationFields {
        &self.0
    }
}

impl TryFrom<LeaseObservationFields> for LeaseObservation {
    type Error = PublicError;

    fn try_from(fields: LeaseObservationFields) -> Result<Self, Self::Error> {
        if fields.generation < FIRST_LEASE_GENERATION
            || fields.expires_at_ms <= 0
            || !metadata(&fields.holder_id, MAX_WRITER_ID_BYTES)
            || fields
                .controller_instance_id
                .as_ref()
                .is_some_and(|id| !metadata(id, MAX_WRITER_ID_BYTES))
        {
            return Err(PublicError::InvalidLease);
        }
        Ok(Self(fields))
    }
}

impl From<LeaseObservation> for LeaseObservationFields {
    fn from(value: LeaseObservation) -> Self {
        value.0
    }
}
