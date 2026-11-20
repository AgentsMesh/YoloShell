use crate::{BrokerHubError, constants::*};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrokerHubProfile {
    pub host_id: String,
    pub profile_authority_sha256: String,
    pub config_revision: String,
}

impl BrokerHubProfile {
    pub fn validate(&self) -> Result<(), BrokerHubError> {
        let host = !self.host_id.is_empty()
            && self.host_id.len() <= MAX_HOST_ID_BYTES
            && self
                .host_id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'));
        if !host || !digest(&self.profile_authority_sha256) || !digest(&self.config_revision) {
            return Err(BrokerHubError::Profile);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrokerHubConnectionBinding {
    pub profile: BrokerHubProfile,
    pub broker_instance_id: Uuid,
    pub connection_id: Uuid,
    pub connection_generation: u64,
}

impl BrokerHubConnectionBinding {
    pub fn validate(&self) -> Result<(), BrokerHubError> {
        self.profile.validate()?;
        if self.broker_instance_id.is_nil()
            || self.connection_id.is_nil()
            || self.connection_generation == u64::default()
        {
            return Err(BrokerHubError::Binding);
        }
        Ok(())
    }
}

fn digest(value: &str) -> bool {
    value.len() == SHA256_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}
