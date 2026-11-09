use serde::{Deserialize, Serialize};

use crate::constants::TOKEN_PUNCTUATION;
use crate::{
    MAX_NATIVE_SESSION_REFERENCE_BYTES, MAX_PROVIDER_DISPLAY_NAME_BYTES, MAX_PROVIDER_KIND_BYTES,
    MAX_PROVIDER_VERSION_BYTES, ProtocolError,
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "ObservedProviderFields", into = "ObservedProviderFields")]
pub struct ObservedProvider(ObservedProviderFields);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ObservedProviderFields {
    kind: String,
    version: Option<String>,
    native_session_reference: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    display_name: Option<String>,
}

impl ObservedProvider {
    pub fn new(
        kind: String,
        version: Option<String>,
        native_session_reference: Option<String>,
    ) -> Result<Self, ProtocolError> {
        ObservedProviderFields {
            kind,
            version,
            native_session_reference,
            display_name: None,
        }
        .try_into()
    }

    pub fn kind(&self) -> &str {
        &self.0.kind
    }

    pub fn version(&self) -> Option<&str> {
        self.0.version.as_deref()
    }

    /// Opaque provider metadata, never a globally routable identity or a file path.
    pub fn native_session_reference(&self) -> Option<&str> {
        self.0.native_session_reference.as_deref()
    }

    /// Provider-owned human-readable name from the compiled provider catalog.
    pub fn display_name(&self) -> Option<&str> {
        self.0.display_name.as_deref()
    }

    pub fn with_display_name(mut self, display_name: Option<String>) -> Result<Self, ProtocolError> {
        if let Some(value) = &display_name {
            if !valid_text(value, MAX_PROVIDER_DISPLAY_NAME_BYTES) {
                return Err(ProtocolError::InvalidProvider);
            }
        }
        self.0.display_name = display_name;
        Ok(self)
    }
}

impl TryFrom<ObservedProviderFields> for ObservedProvider {
    type Error = ProtocolError;

    fn try_from(fields: ObservedProviderFields) -> Result<Self, Self::Error> {
        if !valid_token(&fields.kind, MAX_PROVIDER_KIND_BYTES)
            || fields
                .version
                .as_deref()
                .is_some_and(|v| !valid_text(v, MAX_PROVIDER_VERSION_BYTES))
            || fields
                .native_session_reference
                .as_deref()
                .is_some_and(|v| !valid_text(v, MAX_NATIVE_SESSION_REFERENCE_BYTES))
            || fields
                .display_name
                .as_deref()
                .is_some_and(|v| !valid_text(v, MAX_PROVIDER_DISPLAY_NAME_BYTES))
        {
            return Err(ProtocolError::InvalidProvider);
        }
        Ok(Self(fields))
    }
}

impl From<ObservedProvider> for ObservedProviderFields {
    fn from(provider: ObservedProvider) -> Self {
        provider.0
    }
}

pub(crate) fn valid_token(value: &str, limit: usize) -> bool {
    !value.is_empty()
        && value.len() <= limit
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || TOKEN_PUNCTUATION.contains(&b))
}

pub(crate) fn valid_text(value: &str, limit: usize) -> bool {
    !value.is_empty() && value.len() <= limit && !value.chars().any(char::is_control)
}
