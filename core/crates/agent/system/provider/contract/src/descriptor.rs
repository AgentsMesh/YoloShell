use crate::capability::{ProviderCapabilities, ProviderTier};
use crate::constants::{
    MAX_PROVIDER_DISPLAY_NAME_BYTES, MAX_PROVIDER_ICON_BYTES, MAX_PROVIDER_MANIFEST_ID_BYTES,
    MAX_PROVIDER_TOKEN_BYTES, PROVIDER_DESCRIPTOR_SCHEMA_VERSION, message,
};
use crate::semantics::{AuthorityKind, LifecycleSemantics, ProjectionPolicy};
use serde::{Deserialize, Serialize};
use yolo_agent_contract::AgentKind;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "ProviderDescriptorFields", into = "ProviderDescriptorFields")]
pub struct ProviderDescriptor(ProviderDescriptorFields);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProviderDescriptorFields {
    schema_version: u16,
    manifest_id: String,
    kind: AgentKind,
    install_token: String,
    conversation_token: String,
    target_tier: ProviderTier,
    capabilities: ProviderCapabilities,
    display_name: String,
    icon: Option<String>,
    authority_kind: AuthorityKind,
    lifecycle_semantics: LifecycleSemantics,
    projection_policy: ProjectionPolicy,
}

impl ProviderDescriptor {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        manifest_id: String,
        kind: AgentKind,
        install_token: String,
        conversation_token: String,
        target_tier: ProviderTier,
        capabilities: ProviderCapabilities,
        display_name: String,
        icon: Option<String>,
        authority_kind: AuthorityKind,
        lifecycle_semantics: LifecycleSemantics,
        projection_policy: ProjectionPolicy,
    ) -> Result<Self, ProviderDescriptorError> {
        if !valid_manifest_id(&manifest_id) {
            return Err(ProviderDescriptorError::Invalid(
                message::MANIFEST_ID,
            ));
        }
        if !valid_token(kind.as_str()) {
            return Err(ProviderDescriptorError::Invalid(message::KIND));
        }
        if !valid_token(&install_token) || !valid_token(&conversation_token) {
            return Err(ProviderDescriptorError::Invalid(message::TOKEN));
        }
        if !valid_text(&display_name, MAX_PROVIDER_DISPLAY_NAME_BYTES) {
            return Err(ProviderDescriptorError::Invalid(message::DISPLAY_NAME));
        }
        if icon
            .as_deref()
            .is_some_and(|value| !valid_token_with_limit(value, MAX_PROVIDER_ICON_BYTES))
        {
            return Err(ProviderDescriptorError::Invalid(message::ICON));
        }
        if !capabilities.contains(target_tier.required()) {
            return Err(ProviderDescriptorError::Invalid(message::CAPABILITY_FLOOR));
        }
        if projection_policy.hook_authoritative && !capabilities.lifecycle {
            return Err(ProviderDescriptorError::Invalid(message::PROJECTION));
        }
        if capabilities.recovery && authority_kind == AuthorityKind::None {
            return Err(ProviderDescriptorError::Invalid(message::CAPABILITY_FLOOR));
        }
        Ok(Self(ProviderDescriptorFields {
            schema_version: PROVIDER_DESCRIPTOR_SCHEMA_VERSION,
            manifest_id,
            kind,
            install_token,
            conversation_token,
            target_tier,
            capabilities,
            display_name,
            icon,
            authority_kind,
            lifecycle_semantics,
            projection_policy,
        }))
    }

    pub fn schema_version(&self) -> u16 {
        self.0.schema_version
    }
    pub fn manifest_id(&self) -> &str {
        &self.0.manifest_id
    }
    pub fn kind(&self) -> &AgentKind {
        &self.0.kind
    }
    pub fn install_token(&self) -> &str {
        &self.0.install_token
    }
    pub fn conversation_token(&self) -> &str {
        &self.0.conversation_token
    }
    pub fn target_tier(&self) -> ProviderTier {
        self.0.target_tier
    }
    pub fn capabilities(&self) -> ProviderCapabilities {
        self.0.capabilities
    }
    pub fn display_name(&self) -> &str {
        &self.0.display_name
    }
    pub fn icon(&self) -> Option<&str> {
        self.0.icon.as_deref()
    }
    pub fn authority_kind(&self) -> AuthorityKind {
        self.0.authority_kind
    }
    pub fn lifecycle_semantics(&self) -> LifecycleSemantics {
        self.0.lifecycle_semantics
    }
    pub fn projection_policy(&self) -> ProjectionPolicy {
        self.0.projection_policy
    }
    pub fn guaranteed_floor(&self) -> ProviderTier {
        self.0.capabilities.floor()
    }
}

impl TryFrom<ProviderDescriptorFields> for ProviderDescriptor {
    type Error = ProviderDescriptorError;

    fn try_from(fields: ProviderDescriptorFields) -> Result<Self, Self::Error> {
        if fields.schema_version != PROVIDER_DESCRIPTOR_SCHEMA_VERSION {
            return Err(ProviderDescriptorError::UnsupportedSchema);
        }
        Self::new(
            fields.manifest_id,
            fields.kind,
            fields.install_token,
            fields.conversation_token,
            fields.target_tier,
            fields.capabilities,
            fields.display_name,
            fields.icon,
            fields.authority_kind,
            fields.lifecycle_semantics,
            fields.projection_policy,
        )
    }
}

impl From<ProviderDescriptor> for ProviderDescriptorFields {
    fn from(value: ProviderDescriptor) -> Self {
        value.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ProviderDescriptorError {
    #[error("{0}")]
    Invalid(&'static str),
    #[error("unsupported provider descriptor schema")]
    UnsupportedSchema,
}

fn valid_manifest_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_PROVIDER_MANIFEST_ID_BYTES
        && value.contains('.')
        && value.split('.').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        })
}

fn valid_token(value: &str) -> bool {
    valid_token_with_limit(value, MAX_PROVIDER_TOKEN_BYTES)
}

fn valid_token_with_limit(value: &str, limit: usize) -> bool {
    !value.is_empty()
        && value.len() <= limit
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'-' | b'_')
        })
}

fn valid_text(value: &str, limit: usize) -> bool {
    !value.is_empty()
        && value.len() <= limit
        && !value.chars().any(char::is_control)
}
