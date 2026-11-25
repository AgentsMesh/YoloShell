pub use yolo_agent_adapter_contract::DetectionContext;
pub use yolo_agent_contract::{AgentBinding, AgentKind};
pub use yolo_agent_core::constants;
pub use yolo_agent_provider_contract::{
    AuthorityKind, InteractionSemantics, InterruptDelivery, LifecycleSemantics, ProjectionPolicies,
    ProjectionPolicy, PromptSubmit, ProviderCapabilities, ProviderDescriptor, ProviderTier,
};
pub use yolo_agent_provider_runtime::{
    ProviderRuntime, ProviderRuntimeRegistry, TurnEndKind, TurnEndProbe, TurnEndRecord,
};
pub use yolo_process as process;

#[path = "adapter/mod.rs"]
pub mod adapter;
#[path = "builtin.rs"]
mod builtin;

pub use adapter::{
    AgentAdapter, AgentRegistry, EvidenceDiscoveryPolicy, SessionGenerationPolicy,
    agent_registry_from_definitions, agent_registry_from_runtime, builtin_agent_registry,
};
pub use builtin::{
    builtin_conversation_sources, builtin_install_spec, builtin_install_specs,
    builtin_launch_policies, builtin_projection_policies, builtin_provider_runtime,
};

#[cfg(test)]
#[path = "../test_support/binding_schema_constants.rs"]
mod binding_schema_test_constants;
#[cfg(test)]
#[path = "../unit_tests/binding_schema_tests.rs"]
mod binding_schema_tests;
#[cfg(test)]
#[path = "../unit_tests/builtin_runtime_tests.rs"]
mod builtin_runtime_tests;
#[cfg(test)]
#[path = "../unit_tests/composition_tests.rs"]
mod composition_tests;
