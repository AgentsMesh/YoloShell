//! Grok provider identity strings. This centralized constants
//! module is the single definition of the names the provider exposes to the
//! runtime, the manifest catalog and the implementation crates. It stays
//! dependency-light because client discovery links it.
use yolo_agent_contract::AgentKind;
use yolo_plugin_provider::{PROVIDER_SCHEMA_VERSION, ProviderDefinition};

pub const PROVIDER_ID: &str = "yolo.provider.grok";
pub const KIND: &str = "grok";
pub const CONVERSATION_TOKEN: &str = KIND;
pub const DISPLAY_NAME: &str = "Grok Build";

pub fn kind() -> AgentKind {
    AgentKind::Other(KIND.into())
}

pub fn definition() -> ProviderDefinition {
    ProviderDefinition::new(PROVIDER_SCHEMA_VERSION, PROVIDER_ID.into(), kind())
        .expect("built-in provider definition must be valid")
}
