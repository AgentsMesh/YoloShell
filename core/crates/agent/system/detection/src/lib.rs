pub use yolo_agent_adapter_contract::{
    AgentAdapter, DetectionContext, DetectionEnvironment, EvidenceDiscoveryPolicy,
    LaunchEnvironment, SessionGenerationPolicy,
};
pub use yolo_agent_contract::*;
pub use yolo_agent_core::{constants, files};
pub use yolo_agent_process::{
    FencedDescendants, ProcessScanFuture, ProcessScanObservation, ProcessScanner, ProcessSource,
    RuntimeProcessSource,
};
pub use yolo_agent_adapter_registry::AgentRegistry;
pub use yolo_cli as cli;
pub use yolo_local_unix as local_unix;
pub use yolo_process as process;
pub use yolo_terminal_model as model;
pub use yolo_wire as protocol;

#[cfg(test)]
#[path = "../test_support/extensibility_constants.rs"]
mod extensibility_constants;
#[cfg(any(test, feature = "adversarial-test-hooks"))]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../test_support/process_snapshot_adversarial_constants.rs"]
pub(crate) mod process_snapshot_fault;

#[path = "detector/mod.rs"]
mod detector;
#[path = "environment.rs"]
mod environment;
#[path = "isolated_process_scanner.rs"]
mod isolated_process_scanner;
#[path = "process_snapshot.rs"]
mod process_snapshot;
#[path = "scan.rs"]
mod scan;

pub mod agent {
    pub use yolo_agent_adapter_contract::{
        AgentAdapter, DetectionContext, EvidenceDiscoveryPolicy, SessionGenerationPolicy,
    };
    pub use yolo_agent_contract::*;
    pub use yolo_agent_core::{constants, files};

    pub mod adapter {
        pub use yolo_agent_adapter_contract::{EvidenceDiscoveryPolicy, SessionGenerationPolicy};
    }

    #[cfg(test)]
    pub use yolo_agent_adapter_registry::AgentRegistry;
    #[cfg(test)]
    pub mod claude {
        pub use yolo_claude::claude::*;
    }
    #[cfg(test)]
    pub mod codex {
        pub use yolo_codex::codex::*;
    }
    #[cfg(test)]
    pub mod launch {
        pub use yolo_agent_launch::*;
    }
}

pub use detector::{AgentDetector, binding_display_title_with_registry};
pub use environment::validate_launch_environment;
pub use isolated_process_scanner::IsolatedProcessScanner;
pub use process_snapshot::{
    ProcessSnapshotOutcome, ProcessSnapshotReceipt, read_process_snapshot,
    start_process_snapshot_parent_watch, write_system_process_snapshot,
};
pub use scan::{AgentScanError, detect_scan};

#[cfg(test)]
pub use yolo_application_support::constants as application_constants;
#[cfg(test)]
pub mod launch {
    pub use yolo_agent_launch::*;
}
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/extensibility_tests.rs"]
mod extensibility_tests;

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/environment_process_tests.rs"]
mod environment_process_tests;

#[cfg(all(test, feature = "yolo-unit-tests"))]
include!("../unit_tests/module_wiring.rs");
