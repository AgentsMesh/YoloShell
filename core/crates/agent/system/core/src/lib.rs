#[path = "argv.rs"]
#[doc(hidden)]
pub mod argv;
#[path = "constants.rs"]
pub mod constants;
#[path = "containment_marker.rs"]
mod containment_marker;
#[path = "files.rs"]
#[doc(hidden)]
pub mod files;
#[path = "matching.rs"]
#[doc(hidden)]
pub mod matching;
#[path = "process_environment.rs"]
#[doc(hidden)]
pub mod process_environment;
#[path = "shell.rs"]
pub mod shell;
#[path = "validation.rs"]
#[doc(hidden)]
pub mod validation;

pub use containment_marker::process_environment_value;
#[cfg(test)]
#[path = "../test_support/constants.rs"]
mod provider_fixture;
pub use yolo_agent_adapter_contract::{
    AgentAdapter, DetectionContext, DetectionEnvironment, EvidenceDiscoveryPolicy,
    LaunchEnvironment, SessionGenerationPolicy, validate_launch_environment,
};
pub use yolo_agent_contract::{
    AgentBinding, AgentEvidence, AgentKind, AgentRecoveryAvailability, AgentState, DetectionResult,
    ObservedProcess, ResumePlan,
};
pub use yolo_hooks_runtime as hooks;
