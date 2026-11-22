pub use yolo_agent_adapter_contract as adapter;
pub use yolo_agent_adapter_contract::{DetectionContext, DetectionEnvironment};
pub use yolo_agent_contract::{
    AgentBinding, AgentEvidence, AgentKind, AgentState, ObservedProcess, ResumePlan,
};
pub use yolo_agent_core::{constants, files, matching, validation};
pub use yolo_codex_hook_contract as codex_hooks;
pub use yolo_crypto::serde_uuid;
pub use yolo_paths as paths;
pub use yolo_paths::namespace_lease;
pub use yolo_terminal_model as model;

#[path = "mod.rs"]
pub mod codex;

pub mod agent {
    #[cfg(test)]
    pub(crate) use yolo_agent_adapter_contract::{AgentAdapter, DetectionContext};
    #[cfg(test)]
    pub(crate) use yolo_agent_detection::AgentDetector;
    #[cfg(test)]
    pub(crate) use yolo_agent_providers::AgentRegistry;
    pub use yolo_agent_contract::*;
    pub use yolo_agent_core::{constants, hooks, matching};
}

pub use codex::*;
