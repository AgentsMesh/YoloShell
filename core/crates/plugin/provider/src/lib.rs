//! Provider declarations shared by extension discovery and the agent registry.
use serde::{Deserialize, Serialize};
use yolo_agent_contract::AgentKind;

#[path = "schema_constants.rs"]
mod schema_constants;
pub use schema_constants::SCHEMA_VERSION_FIELD;

pub const PROVIDER_SCHEMA_VERSION: u16 = 1;
pub const MAX_PROVIDER_ID_BYTES: usize = 96;
pub const MAX_PROVIDER_KIND_BYTES: usize = 96;
pub const INVALID_DEFINITION: &str = "invalid provider definition";
pub const UNSUPPORTED_DEFINITION: &str = "unsupported provider definition";

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ProviderError {
    #[error("{INVALID_DEFINITION}")]
    Invalid,
    #[error("{UNSUPPORTED_DEFINITION}")]
    Unsupported,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "ProviderFields", into = "ProviderFields")]
pub struct ProviderDefinition(ProviderFields);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProviderFields {
    schema_version: u16,
    id: String,
    kind: AgentKind,
}

impl ProviderDefinition {
    pub fn new(schema_version: u16, id: String, kind: AgentKind) -> Result<Self, ProviderError> {
        if schema_version != PROVIDER_SCHEMA_VERSION || !valid_id(&id) || !valid_kind(kind.as_str())
        {
            return Err(ProviderError::Invalid);
        }
        Ok(Self(ProviderFields {
            schema_version,
            id,
            kind,
        }))
    }

    pub fn schema_version(&self) -> u16 {
        self.0.schema_version
    }

    pub fn id(&self) -> &str {
        &self.0.id
    }

    pub fn kind(&self) -> &AgentKind {
        &self.0.kind
    }
}

impl TryFrom<ProviderFields> for ProviderDefinition {
    type Error = ProviderError;

    fn try_from(fields: ProviderFields) -> Result<Self, Self::Error> {
        Self::new(fields.schema_version, fields.id, fields.kind)
    }
}

impl From<ProviderDefinition> for ProviderFields {
    fn from(value: ProviderDefinition) -> Self {
        value.0
    }
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.contains('.')
        && value.len() <= MAX_PROVIDER_ID_BYTES
        && value.split('.').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        })
}

fn valid_kind(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_PROVIDER_KIND_BYTES
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
        })
}

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;
