pub use yolo_agent_adapter_contract::DetectionContext;
pub use yolo_agent_contract::*;
pub use yolo_agent_core::constants;
pub use yolo_agent_providers::AgentRegistry;
#[cfg(test)]
pub use yolo_application_support::constants as runtime_constants;
pub use yolo_claude_custody::claude_settings_artifact;
pub use yolo_claude_hooks as claude_hooks;
#[cfg(test)]
pub use yolo_codex::codex_hooks;
pub use yolo_paths as paths;
pub use yolo_paths::namespace_lease;
pub use yolo_process as process;
pub use yolo_terminal_model as model;

pub mod claude {
    pub use yolo_claude::claude::*;
}

pub mod codex {
    pub use yolo_codex::codex::*;
}

#[path = "store/mod.rs"]
pub mod store;

pub mod agent {
    #[cfg(test)]
    pub use crate::AgentBindingStore;
    pub use yolo_agent_contract::*;
    pub use yolo_agent_core::constants;
    #[cfg(test)]
    pub use yolo_agent_detection::AgentDetector;
    #[cfg(test)]
    pub use yolo_hooks_runtime as hooks;
    pub mod claude {
        pub use yolo_claude::claude::*;
    }

    pub mod codex {
        pub use yolo_codex::codex::*;
    }

    pub mod adapter {
        pub use yolo_agent_adapter_contract::SessionGenerationPolicy;
    }
}

pub use store::AgentBindingStore;

#[cfg(any(test, feature = "runtime-component-tests"))]
#[path = "../test_support/runtime_api.rs"]
mod runtime_test_api;

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/store_error_tests.rs"]
mod store_error_tests;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/store_recovery_tests.rs"]
mod store_recovery_tests;
