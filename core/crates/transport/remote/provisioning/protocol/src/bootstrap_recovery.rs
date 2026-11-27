use crate::{
    BootstrapError, ENROLLMENT_RECOVERY_SCHEMA, RemoteBootstrapReceipt, RemoteBootstrapRequest,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum EnrollmentOutcome {
    Absent {},
    Unknown {},
    Superseded {},
    Ready { receipt: RemoteBootstrapReceipt },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    try_from = "EnrollmentRecoveryFields",
    into = "EnrollmentRecoveryFields"
)]
pub struct EnrollmentRecovery(EnrollmentRecoveryFields);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnrollmentRecoveryFields {
    pub schema_version: u16,
    pub request_id: Uuid,
    pub outcome: EnrollmentOutcome,
}
impl EnrollmentRecovery {
    pub fn new(
        request: &RemoteBootstrapRequest,
        outcome: EnrollmentOutcome,
    ) -> Result<Self, BootstrapError> {
        let result = Self::try_from(EnrollmentRecoveryFields {
            schema_version: ENROLLMENT_RECOVERY_SCHEMA,
            request_id: request.fields().request_id,
            outcome,
        })?;
        result.validate_for(request)?;
        Ok(result)
    }
    pub fn fields(&self) -> &EnrollmentRecoveryFields {
        &self.0
    }
    pub fn validate_for(&self, request: &RemoteBootstrapRequest) -> Result<(), BootstrapError> {
        if self.0.request_id != request.fields().request_id {
            return Err(BootstrapError::Correlation);
        }
        if let EnrollmentOutcome::Ready { receipt } = &self.0.outcome {
            receipt.validate_for(request)?;
        }
        Ok(())
    }
}
impl TryFrom<EnrollmentRecoveryFields> for EnrollmentRecovery {
    type Error = BootstrapError;
    fn try_from(fields: EnrollmentRecoveryFields) -> Result<Self, Self::Error> {
        if fields.schema_version != ENROLLMENT_RECOVERY_SCHEMA {
            return Err(BootstrapError::Schema);
        }
        if fields.request_id.is_nil() {
            return Err(BootstrapError::RequestIdentity);
        }
        if let EnrollmentOutcome::Ready { receipt } = &fields.outcome {
            if receipt.fields().request_id != fields.request_id {
                return Err(BootstrapError::Correlation);
            }
        }
        Ok(Self(fields))
    }
}
impl From<EnrollmentRecovery> for EnrollmentRecoveryFields {
    fn from(value: EnrollmentRecovery) -> Self {
        value.0
    }
}
