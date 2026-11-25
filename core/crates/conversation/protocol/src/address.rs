use crate::ConversationError;
use serde::{Deserialize, Serialize};
use yolo_crypto::Sha256Digest;
use yolo_hub_identity::{AgentTarget, HubId, RuntimeTarget};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConversationAddress {
    pub hub_id: HubId,
    pub conversation_id: Sha256Digest,
}

/// A conversation can survive several executions; an association always names exact executions.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConversationBinding {
    pub agent: AgentTarget,
    pub runtime: RuntimeTarget,
}

pub use crate::constants::provider_tokens::{
    CLAUDE as CLAUDE_PROVIDER_TOKEN, CODEX as CODEX_PROVIDER_TOKEN, GEMINI as GEMINI_PROVIDER_TOKEN,
    GROK as GROK_PROVIDER_TOKEN, QWEN as QWEN_PROVIDER_TOKEN,
};
pub const MAX_CONVERSATION_PROVIDER_BYTES: usize = 96;

/// Open provider token. Existing values stay byte-identical; unknown providers
/// are rejected by the conversation service only when no parser is registered.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ConversationProvider(String);

impl ConversationProvider {
    pub fn codex() -> Self {
        Self(CODEX_PROVIDER_TOKEN.into())
    }

    pub fn claude_code() -> Self {
        Self(CLAUDE_PROVIDER_TOKEN.into())
    }

    pub fn qwen_code() -> Self {
        Self(QWEN_PROVIDER_TOKEN.into())
    }

    pub fn new(value: impl Into<String>) -> Result<Self, ConversationError> {
        let value = value.into();
        if value.is_empty()
            || value.len() > MAX_CONVERSATION_PROVIDER_BYTES
            || !value.bytes().all(|byte| {
                byte.is_ascii_lowercase()
                    || byte.is_ascii_digit()
                    || matches!(byte, b'-' | b'_')
            })
        {
            return Err(ConversationError::InvalidRequest);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ConversationProvider {
    type Error = ConversationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<ConversationProvider> for String {
    fn from(value: ConversationProvider) -> Self {
        value.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConversationDescriptor {
    pub address: ConversationAddress,
    pub binding: ConversationBinding,
    pub provider: ConversationProvider,
    pub provider_session_id: String,
    pub source: ConversationSource,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConversationSource {
    NativeTranscript,
}
