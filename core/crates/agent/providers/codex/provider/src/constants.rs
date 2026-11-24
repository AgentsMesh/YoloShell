//! Codex provider identity strings. This centralized constants
//! module is the single definition of the names the provider exposes to the
//! runtime, the manifest catalog and the implementation crates. It stays
//! dependency-light because client discovery links it.
use yolo_agent_contract::AgentKind;
use yolo_plugin_provider::{PROVIDER_SCHEMA_VERSION, ProviderDefinition};

pub mod managed_launch;
pub const NO_DAEMON: &str = "--no-daemon";
pub const LEGACY_SETTING: &str = "features.daemon_auto_start=false";
pub const PREVIOUS_START_COMMAND: &str = "codex --config features.daemon_auto_start=false";
pub const RESUME_ARGUMENT: &str = "resume";
pub const CWD_ARGUMENT: &str = "-C";
pub const DATA_HOME_ENV: &str = "CODEX_HOME";

pub const PROVIDER_ID: &str = "yolo.provider.codex";
pub const KIND: &str = "codex";
pub const CONVERSATION_TOKEN: &str = KIND;
pub const DISPLAY_NAME: &str = "Codex";
pub const INSTALL_TOKEN: &str = "codex";

pub fn kind() -> AgentKind {
    AgentKind::Codex
}

pub fn definition() -> ProviderDefinition {
    ProviderDefinition::new(PROVIDER_SCHEMA_VERSION, PROVIDER_ID.into(), kind())
        .expect("built-in provider definition must be valid")
}
