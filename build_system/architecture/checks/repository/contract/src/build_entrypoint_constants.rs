pub(super) const CORE_PACKAGE: &str = "core";
pub(super) const INTEGRATION_PACKAGE: &str = "integration_tests/runtime";
pub(super) const BUILD_FILE: &str = "BUILD.bazel";
pub(super) const STARLARK_EXTENSION: &str = ".bzl";
pub(super) const BAZEL_EXTENSION: &str = ".bazel";
pub(super) const NAME_ATTRIBUTE: &str = "name";
pub(super) const RETIRED_MACRO_FILES: &[&str] = &[
    "build_system/bazel/rust/runtime_integration_aliases.bzl",
    "build_system/bazel/rust/version_fixture.bzl",
];
pub(super) const CORE_FORWARDERS: &[&str] = &[
    "catalog_stream_process_test",
    "runtime_catalog_test",
    "runtime_journal_test",
    "runtime_protocol_test",
    "runtime_resilience_test",
    "codex-version-fixture",
    "codex-live-peer-fixture",
];
pub(super) const INTEGRATION_FORWARDERS: &[&str] = &[
    "catalog_stream_process_test",
    "failed_rearm_process_tests",
    "production_binary_audit",
    "runtime_catalog_test",
    "runtime_journal_test",
    "runtime_protocol_test",
    "socket_root_configuration_test",
    "startup_handoff_process_tests",
];
pub(super) const MESSAGE: &str = "moved tests and fixtures must use their owning Bazel package without historical forwarding targets";
