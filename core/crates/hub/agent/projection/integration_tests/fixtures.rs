use super::constants::*;
use std::path::PathBuf;
use yolo_agent_contract::AgentKind;
use yolo_agent_provider_contract::{ProjectionPolicies, ProjectionPolicy};
use yolo_hub_agent_projection::AgentProjection;
use yolo_hub_identity::HubId;
use yolo_paths::directory::PrivateDirectory;

pub fn directory() -> (tempfile::TempDir, PathBuf, HubId) {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().canonicalize().unwrap().join(TEST_DIRECTORY);
    PrivateDirectory::ensure(&path).unwrap();
    (root, path, HubId::new())
}

pub fn hook_policies() -> ProjectionPolicies {
    let mut policies = ProjectionPolicies::none();
    policies.insert(AgentKind::Codex, ProjectionPolicy::HOOK_AUTHORITATIVE);
    policies.insert(AgentKind::ClaudeCode, ProjectionPolicy::HOOK_AUTHORITATIVE);
    policies
}

pub fn open(path: &std::path::Path, hub: HubId) -> AgentProjection {
    open_with(path, hub, hook_policies())
}

pub fn open_with(
    path: &std::path::Path,
    hub: HubId,
    policies: ProjectionPolicies,
) -> AgentProjection {
    AgentProjection::open_with_policies(PrivateDirectory::open(path).unwrap(), hub, policies)
        .unwrap()
}
