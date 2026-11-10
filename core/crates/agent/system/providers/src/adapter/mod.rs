//! Provider composition helpers for the mechanism adapter registry. Provider
//! policy lives in the provider packages; this module only injects their
//! adapters and environment keys into the registry.
use super::builtin_provider_runtime;
use anyhow::Result;
pub use yolo_agent_adapter_contract::{
    AgentAdapter, EvidenceDiscoveryPolicy, SessionGenerationPolicy,
};
pub use yolo_agent_adapter_registry::AgentRegistry;
use yolo_agent_provider_runtime::ProviderRuntimeRegistry;
use yolo_plugin_provider::ProviderDefinition;

const BUILTIN_REGISTRY_ERROR: &str = "built-in adapter registry must be valid";

pub fn builtin_agent_registry() -> AgentRegistry {
    agent_registry_from_runtime(&builtin_provider_runtime())
}

pub fn agent_registry_from_runtime(runtime: &ProviderRuntimeRegistry) -> AgentRegistry {
    AgentRegistry::from_shared_adapters_and_keys(
        runtime.shared_adapters().collect(),
        runtime.process_environment_keys(),
    )
    .expect(BUILTIN_REGISTRY_ERROR)
}

pub fn agent_registry_from_definitions(
    definitions: impl IntoIterator<Item = ProviderDefinition>,
) -> Result<AgentRegistry> {
    let runtime = builtin_provider_runtime();
    let mut adapters = Vec::new();
    let mut environment_keys = std::collections::BTreeSet::new();
    for definition in definitions {
        let provider = runtime.by_manifest_id(definition.id()).ok_or_else(|| {
            anyhow::anyhow!(
                yolo_agent_adapter_contract::constants::message::PROVIDER_DEFINITION_ADAPTER
            )
        })?;
        anyhow::ensure!(
            provider.descriptor().kind() == definition.kind(),
            yolo_agent_adapter_contract::constants::message::PROVIDER_DEFINITION_ADAPTER
        );
        adapters.push(provider.adapter().clone());
        environment_keys.extend(provider.process_environment_keys().iter().copied());
    }
    AgentRegistry::from_shared_adapters_and_keys(adapters, environment_keys)
}
