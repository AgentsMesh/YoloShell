use yolo_hub_local_backend::LocalBackend;
use yolo_hub_state::HubState;
use yolo_runtime::agent::{
    AgentKind,
    provider_contract::{ProjectionPolicies, ProjectionPolicy},
};

pub const AGENT_DIRECTORY: &str = "agents";

/// Mirrors the compiled provider catalog's Hub projection policy for fixtures
/// that exercise native hook authority. Production injects the real catalog at
/// the Hub process composition root.
pub fn fixture_projection_policies() -> ProjectionPolicies {
    let mut policies = ProjectionPolicies::none();
    policies.insert(AgentKind::Codex, ProjectionPolicy::HOOK_AUTHORITATIVE);
    policies.insert(AgentKind::ClaudeCode, ProjectionPolicy::HOOK_AUTHORITATIVE);
    policies
}

pub fn with_fixture_agents(backend: LocalBackend, state: &HubState) -> LocalBackend {
    let directory = state.directory().unwrap();
    let agents = directory
        .ensure_descendant(&directory.path().join(AGENT_DIRECTORY))
        .unwrap();
    backend
        .with_agents(agents, fixture_projection_policies())
        .unwrap()
}
