use crate::{AccessConfiguration, AccessError, FIRST_ACCESS_REVISION};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClientAccessChange {
    #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
    pub request_id: Uuid,
    pub expected_revision: u64,
    pub configuration: AccessConfiguration,
}

impl ClientAccessChange {
    pub fn validate(&self) -> Result<(), AccessError> {
        self.configuration.validate()?;
        if self.request_id.is_nil()
            || self.expected_revision < FIRST_ACCESS_REVISION
            || self
                .expected_revision
                .checked_add(crate::ACCESS_REVISION_STEP)
                != Some(self.configuration.revision)
        {
            return Err(AccessError::Configuration);
        }
        Ok(())
    }
}
