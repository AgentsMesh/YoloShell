pub const RUST_RESILIENCE_TEST_TARGET: &str = "runtime_resilience_test";
pub const RUST_HOST_TEST_TARGETS: &[&str] = &["host_unit_tests", "host_integration_tests"];
pub const RUST_REMOTE_TEST_TARGETS: &[&str] = &["remote_unit_tests", "remote_integration_tests"];
pub const ROOT_IDENTITY_MARKER: &str = "root_identity";
pub const TEST_ISOLATION_FORBIDDEN_MARKERS: &[&str] =
    &["remove_dir_all", "rm -rf", "Command::new(\"cargo\")"];
pub const SCM_RIGHTS_RUST_CONTRACT_RULE: &str = "scm_rights_descriptor_rust_contract";
