#[path = "deployment/artifact.rs"]
mod artifact;
#[path = "deployment/artifact_format.rs"]
mod artifact_format;
#[path = "deployment/orchestrator.rs"]
mod orchestrator;
#[path = "deployment/plan.rs"]
mod plan;

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../integration_tests/deployment/artifact_format_tests.rs"]
mod artifact_format_tests;

pub use artifact::{RemoteArchitecture, RemotePlatform, RuntimeArtifact};
pub use orchestrator::{RuntimeDeploymentResult, RuntimeInstaller};

use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct RuntimeStageRequest {
    pub(crate) directory_path: String,
    pub(crate) stable_path: String,
    pub(crate) release_path: String,
    pub(crate) temporary_path: String,
    pub(crate) bytes: Arc<[u8]>,
    pub(crate) sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeStageResult {
    pub already_active: bool,
    pub had_previous_runtime: bool,
}

#[derive(Debug, Clone)]
pub struct RuntimeActivationRequest {
    pub(crate) stable_path: String,
    pub(crate) release_path: String,
    pub(crate) temporary_link: String,
    pub(crate) expected_bytes: u64,
    pub(crate) sha256: String,
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../integration_tests/deployment/tests.rs"]
mod tests;
