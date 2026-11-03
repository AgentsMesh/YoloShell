#[path = "../test_support/runtime_manifest_fixture.rs"]
mod runtime_manifest_fixture;
#[path = "../test_support/support.rs"]
mod support;
use yolo_release_gate_test_support::constants as test_constants;

#[path = "audit_runner_tests.rs"]
mod audit_runner_tests;
mod coverage_custody_tests;
#[path = "coverage_report_tests.rs"]
mod coverage_report_tests;
#[path = "gate_runner_tests.rs"]
mod gate_runner_tests;
#[path = "linked_worktree_tests.rs"]
mod linked_worktree_tests;
#[path = "linux_cgroup_gate_command_tests.rs"]
mod linux_cgroup_gate_command_tests;
#[path = "linux_gate_command_tests.rs"]
mod linux_gate_command_tests;
mod linux_runtime_output_tests;
#[path = "linux_runtime_contract_tests.rs"]
mod linux_runtime_contract_tests;
#[path = "private_git_tests.rs"]
mod private_git_tests;
mod snapshot_artifact_tests;
mod snapshot_cleanup_tests;
mod snapshot_manifest_permission_tests;
#[path = "snapshot_namespace_tests.rs"]
mod snapshot_namespace_tests;
mod snapshot_source_access_tests;
#[path = "snapshot_tests.rs"]
mod snapshot_tests;
