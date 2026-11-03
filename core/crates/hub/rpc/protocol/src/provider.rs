use serde::{Deserialize, Serialize};
use yolo_hub_terminal_protocol::PublicError;

pub const MAX_PROVIDERS: usize = 64;
pub const MAX_PROVIDER_ID_BYTES: usize = 96;
pub const MAX_PROVIDER_NAME_BYTES: usize = 128;

/// One agent provider as known to a machine's launch environment.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderSummary {
    pub provider_id: String,
    pub display_name: String,
    pub available: bool,
    #[serde(default)]
    pub features: ProviderFeatures,
}

/// Safe provider capabilities exposed to clients. Missing fields default for
/// older peers; the values are provider facts, not user input.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ProviderFeatures {
    pub conversation: bool,
    pub submit_prompt: bool,
    pub interrupt: bool,
    pub interaction: ProviderInteraction,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderInteraction {
    #[default]
    None,
    Terminal,
    Typed,
}

impl ProviderSummary {
    pub fn new(
        provider_id: impl Into<String>,
        display_name: impl Into<String>,
        available: bool,
    ) -> Result<Self, PublicError> {
        let value = Self {
            provider_id: provider_id.into(),
            display_name: display_name.into(),
            available,
            features: ProviderFeatures::default(),
        };
        value.validate()?;
        Ok(value)
    }

    pub fn with_features(mut self, features: ProviderFeatures) -> Self {
        self.features = features;
        self
    }

    pub fn validate(&self) -> Result<(), PublicError> {
        if !valid_id(&self.provider_id) || !valid_name(&self.display_name) {
            return Err(PublicError::InvalidMetadata);
        }
        Ok(())
    }
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_PROVIDER_ID_BYTES
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-')
        })
}

fn valid_name(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= MAX_PROVIDER_NAME_BYTES
        && !value.chars().any(char::is_control)
}
