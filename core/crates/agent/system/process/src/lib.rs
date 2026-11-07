pub use yolo_agent_contract::ObservedProcess;
pub use yolo_agent_core::{argv, constants, process_environment};
pub use yolo_process as process;

#[path = "process_scanner.rs"]
mod process_scanner;
#[path = "process/mod.rs"]
mod source;

pub mod agent {
    pub use yolo_agent_core::constants;
    #[cfg(test)]
    pub mod claude {
        pub use yolo_claude::claude::*;
    }
    #[cfg(test)]
    pub mod codex {
        pub use yolo_codex::codex::*;
    }
}

pub use process_scanner::{ProcessScanFuture, ProcessScanObservation, ProcessScanner};
pub use source::{FencedDescendants, ProcessSource, RuntimeProcessSource};

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/process_environment_codex_tests.rs"]
mod process_environment_codex_tests;
