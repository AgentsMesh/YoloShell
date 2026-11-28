pub use yolo_application_support::constants;
pub use yolo_claude_custody::{claude_custody, claude_settings_artifact};
pub use yolo_claude_hooks as claude_hooks;
pub use yolo_cli as cli;
pub use yolo_codex_hooks as codex_hooks;
pub use yolo_qwen_hooks as qwen_hooks;
pub use yolo_crypto::{serde_non_nil_uuid, serde_uuid, sha256_digest};
pub use yolo_journal as journal;
pub use yolo_lease as lease;
pub use yolo_local_unix as local_unix;
pub use yolo_paths as paths;
pub use yolo_paths::coordination;
pub use yolo_paths::namespace_lease;
pub use yolo_persistence_custody::custody_promotion;
pub use yolo_private_acl as private_acl;
pub use yolo_process as process;
pub use yolo_pty_custody as pty_custody;
pub use yolo_session_timing as session_timing;
pub use yolo_terminal_model as model;
pub use yolo_transport_core as ingress;

pub mod protocol {
    pub use yolo_protocol_validation::*;
    pub use yolo_wire::*;
}

pub mod agent {
    pub mod authority {
        pub use yolo_agent_authority_contract::*;
    }

    pub mod claude {
        pub use yolo_claude::claude::*;
    }

    pub mod codex {
        pub use yolo_codex::codex::*;
    }

    pub mod qwen {
        pub use yolo_qwen::qwen::*;
    }

    pub mod launch {
        pub use yolo_agent_launch::*;
    }

    pub use yolo_agent_adapter_contract::{
        AgentAdapter, DetectionContext, validate_launch_environment,
    };
    pub use yolo_agent_contract::*;
    pub use yolo_agent_core::{constants, hooks};
    pub use yolo_agent_detection::{
        AgentDetector, binding_display_title_with_registry, detect_scan,
    };
    pub use yolo_agent_process::{
        FencedDescendants, ProcessSource, RuntimeProcessSource,
    };
    pub use yolo_agent_providers::{AgentRegistry, agent_registry_from_runtime, builtin_agent_registry};
    pub use yolo_agent_store::AgentBindingStore;
}

#[path = "mod.rs"]
pub mod worker;

pub use worker::*;

#[cfg(all(test, target_os = "macos", feature = "yolo-unit-tests"))]
include!("../unit_tests/process_lineage_entrypoint.rs");
#[cfg(all(test, target_os = "macos", feature = "yolo-integration-tests"))]
include!("../integration_tests/process_lineage_entrypoint.rs");

#[cfg(feature = "adversarial-test-hooks")]
pub use yolo_application_test_fixtures::survivor as survivor_test_hook;

#[cfg(test)]
pub use yolo_application_support::acl_fixture;
#[cfg(test)]
pub use yolo_application_test_fixtures::{
    executables as test_executables, tracing as tracing_test_support,
};
#[cfg(test)]
pub use yolo_custody_guardian::custody_guardian;
#[cfg(test)]
pub use yolo_durable_reboot::durable_reboot;
