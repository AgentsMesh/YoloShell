mod audit_runner;
mod audit_runner_result;
pub mod container_command;
pub mod container_mounts;
mod coverage_report;
mod coverage_report_io;
mod gate_command;
mod gate_requirement;
mod gate_runner;
pub mod linux_cgroup_gate_command;
pub mod linux_gate_command;
mod linux_runtime_output;
mod output_namespace;
mod output_validation;
mod private_git;
mod private_git_support;
mod requirements;
mod runtime_manifest_audit;
mod snapshot;
mod snapshot_access;
mod snapshot_support;
mod validation_linux;

pub use audit_runner::audit;
pub use gate_runner::{ProcessExecuting, SystemProcessExecutor, run_gate};
pub use output_namespace::{entry_names, validate_exact_entries, validate_named_entries};
pub use output_validation::{ensure_missing, entry_exists, validate_directory, validate_parent};
pub use requirements::{
    GateCommand, GateKind, GateRequirement, LINUX_AMD64_PROFILE, LINUX_ARM64_PROFILE,
    LinuxGateProfile, PACKAGE_COMMAND, REQUIRED_GATES,
};
pub use runtime_manifest_audit::{
    AUDIT_BUNDLE_OPERATION, AUDIT_MANIFEST_OPERATION, MANIFEST_ARGUMENT,
    audit_bundle_manifest_path, audit_runtime_manifest_path,
};
pub use snapshot::GateSnapshot;
pub use validation_linux::{find as find_linux_runtime, validate as validate_linux_runtimes};

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/lib.rs"]
mod integration_tests;

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;
