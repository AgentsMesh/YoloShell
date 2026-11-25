pub use yolo_agent_adapter_contract as adapter;
pub use yolo_agent_adapter_contract::{DetectionContext, DetectionEnvironment};
pub use yolo_agent_contract::{
    AgentBinding, AgentEvidence, AgentKind, AgentState, ObservedProcess, ResumePlan,
};
pub use yolo_agent_core::{constants, files, matching, validation};
#[cfg(not(all(test, feature = "yolo-unit-tests")))]
pub use yolo_claude_hook_contract as claude_hooks;
#[cfg(all(test, feature = "yolo-unit-tests"))]
pub use yolo_claude_hooks as claude_hooks;
pub use yolo_crypto::serde_uuid;
pub use yolo_paths as paths;
pub use yolo_paths::namespace_lease;
pub use yolo_process as process;
pub use yolo_terminal_model as model;
#[cfg(all(test, feature = "yolo-unit-tests"))]
pub use yolo_claude_custody::claude_settings_artifact;
#[cfg(all(test, feature = "yolo-unit-tests"))]
pub use yolo_crypto::sha256_digest;

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../test_support/binding_store.rs"]
mod test_binding_store;

#[path = "mod.rs"]
pub mod claude;

pub mod agent {
    #[cfg(all(test, feature = "yolo-unit-tests"))]
    pub(crate) use crate::test_binding_store::AgentBindingStore;
    #[cfg(all(test, feature = "yolo-unit-tests"))]
    pub use yolo_agent_adapter_contract::DetectionContext;
    #[cfg(all(test, feature = "yolo-unit-tests"))]
    pub use yolo_agent_detection::AgentDetector;
    #[cfg(all(test, feature = "yolo-unit-tests"))]
    pub use yolo_agent_providers::AgentRegistry;
    pub use crate::claude;
    pub use yolo_agent_adapter_contract::AgentAdapter;
    pub use yolo_agent_contract::*;
    pub use yolo_agent_core::{constants, files};
}

pub use claude::*;
