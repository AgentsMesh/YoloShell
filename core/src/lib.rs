extern crate self as yolo_runtime;

pub mod agent {
    pub use yolo_agent_adapter_contract as adapter;
    pub use yolo_agent_adapter_contract::{
        AgentAdapter, DetectionContext, DetectionEnvironment, EvidenceDiscoveryPolicy,
        LaunchEnvironment, SessionGenerationPolicy, validate_launch_environment,
    };
    pub use yolo_agent_contract::*;
    pub use yolo_agent_core::{
        constants, files, hooks, process_environment, process_environment_value, shell,
    };
    pub use yolo_agent_detection::{
        AgentDetector, IsolatedProcessScanner, ProcessScanFuture, ProcessScanObservation,
        ProcessScanner, detect_scan, start_process_snapshot_parent_watch,
        write_system_process_snapshot,
    };
    pub use yolo_agent_launch as launch;
    pub use yolo_agent_process::{FencedDescendants, ProcessSource, RuntimeProcessSource};
    pub use yolo_agent_provider_contract as provider_contract;
    pub use yolo_agent_provider_runtime::{
        HookClient, ProviderRuntime, ProviderRuntimeError, ProviderRuntimeRegistry,
    };
    pub use yolo_agent_providers::{
        AgentRegistry, agent_registry_from_runtime, builtin_agent_registry,
        builtin_projection_policies, builtin_provider_runtime,
    };
    pub use yolo_agent_store::AgentBindingStore;
    pub use yolo_claude::claude;
    pub use yolo_codex::codex;
}

pub use yolo_application_support::constants;
pub use yolo_application_support::isolated_test_child;
pub use yolo_application_support::time_constants;
pub use yolo_catalog as catalog;
pub use yolo_claude_custody::{claude_custody, claude_settings_artifact};
pub use yolo_claude_hook_bridge as claude_hook_bridge;
pub use yolo_claude_hooks as claude_hooks;
pub use yolo_cli as cli;
pub use yolo_codex_hooks as codex_hooks;
pub use yolo_qwen_hooks as qwen_hooks;
pub use yolo_gemini_hooks as gemini_hooks;
pub use yolo_grok_hooks as grok_hooks;
pub use yolo_hook_codec as hook_codec;
pub use yolo_hook_ingress as hook_ingress;
pub use yolo_custody_guardian as custody_guardian;
pub use yolo_durable_reboot as durable_reboot;
pub use yolo_host as host;
pub use yolo_journal as journal;
pub use yolo_lease as lease;
pub use yolo_local_unix as local_unix;
pub use yolo_maintenance as maintenance;
pub use yolo_paths::coordination;
#[path = "model.rs"]
pub mod model;
pub use yolo_paths as paths;
pub use yolo_paths::namespace_lease;
pub use yolo_persistence_custody::custody_promotion;
#[cfg(target_os = "macos")]
pub use yolo_persistence_custody::custody_retirement;
pub use yolo_process as process;
#[path = "protocol.rs"]
pub mod protocol;
pub use yolo_pty_custody as pty_custody;
pub use yolo_worker as worker;

pub use host::{Host, HostConfig};
pub use model::*;
pub use protocol::{ErrorBody, EventEnvelope, RequestEnvelope, ResponseEnvelope};
