use serde::{Deserialize, Serialize};
use uuid::Uuid;
use yolo_hub_identity::RuntimeTarget;

use crate::{FIRST_LEASE_GENERATION, MAX_LEASE_TTL_MS, PublicError, SecretCapability, validation};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WriterPrincipal {
    #[serde(
        serialize_with = "validation::serialize_id",
        deserialize_with = "validation::deserialize_id"
    )]
    pub client_id: Uuid,
    #[serde(
        serialize_with = "validation::serialize_id",
        deserialize_with = "validation::deserialize_id"
    )]
    pub controller_instance_id: Uuid,
}

impl WriterPrincipal {
    pub fn validate(self) -> Result<(), PublicError> {
        validation::request_id(self.client_id)?;
        validation::request_id(self.controller_instance_id)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "LeaseGrantFields", into = "LeaseGrantFields")]
pub struct LeaseGrant(LeaseGrantFields);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LeaseGrantFields {
    pub target: RuntimeTarget,
    pub principal: WriterPrincipal,
    pub generation: u64,
    pub capability: SecretCapability,
    pub expires_at_ms: i64,
    pub ttl_ms: u64,
    pub renew_after_ms: u64,
}

impl LeaseGrant {
    pub fn new(fields: LeaseGrantFields) -> Result<Self, PublicError> {
        fields.try_into()
    }

    pub fn fields(&self) -> &LeaseGrantFields {
        &self.0
    }
}

impl TryFrom<LeaseGrantFields> for LeaseGrant {
    type Error = PublicError;

    fn try_from(fields: LeaseGrantFields) -> Result<Self, Self::Error> {
        fields.principal.validate()?;
        if fields.generation < FIRST_LEASE_GENERATION
            || fields.expires_at_ms <= 0
            || fields.ttl_ms == 0
            || fields.ttl_ms > MAX_LEASE_TTL_MS
            || fields.renew_after_ms == 0
            || fields.renew_after_ms >= fields.ttl_ms
        {
            return Err(PublicError::InvalidLease);
        }
        Ok(Self(fields))
    }
}

impl From<LeaseGrant> for LeaseGrantFields {
    fn from(value: LeaseGrant) -> Self {
        value.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum LeaseMode {
    Acquire {},
    TakeControl {
        #[serde(deserialize_with = "validation::deserialize_generation")]
        observed_generation: u64,
    },
}

impl LeaseMode {
    pub fn validate(self) -> Result<(), PublicError> {
        if matches!(self, Self::TakeControl { observed_generation } if observed_generation < FIRST_LEASE_GENERATION)
        {
            return Err(PublicError::InvalidLease);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LeaseRequest {
    #[serde(
        serialize_with = "validation::serialize_id",
        deserialize_with = "validation::deserialize_id"
    )]
    pub request_id: Uuid,
    pub target: RuntimeTarget,
    pub principal: WriterPrincipal,
    pub mode: LeaseMode,
}

impl LeaseRequest {
    pub fn validate(self) -> Result<(), PublicError> {
        validation::request_id(self.request_id)?;
        self.principal.validate()?;
        self.mode.validate()
    }
}
