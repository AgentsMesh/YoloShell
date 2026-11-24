pub const CAPNP_DEFINITIONS: &str = "build_system/bazel/capnp/defs.bzl";
pub const CAPNP_GENERATION: &str = "build_system/bazel/capnp/generation.bzl";
pub const CAPNP_LIBRARY_MACRO: &str = "capnp_rust_library";
pub const CAPNP_SOURCES_MACRO: &str = "capnp_rust_sources";
pub const CAPNP_METHODS: &[&str] = &["replace", "args", "add", "add_all", "run"];
pub const CAPNP_BUILTINS: &[&str] = &["len", "OutputGroupInfo"];
pub const MACRO_BUILTINS: &[&str] = &[
    "depset",
    "load",
    "rule",
    "Label",
    "DefaultInfo",
    "RunEnvironmentInfo",
    "configuration_field",
    "dict",
    "fail",
    "str",
    "any",
    "max",
];
pub const MACRO_METHODS: &[&str] = &[
    "alias",
    "test_suite",
    "package_name",
    "write",
    "symlink",
    "declare_file",
    "runfiles",
    "instrumented_files_info",
    "label",
    "label_list",
    "merge",
    "get",
    "items",
    "keys",
    "update",
    "append",
    "format",
    "encode",
    "join",
    "split",
    "startswith",
    "endswith",
];
pub const SAFE_BUILTINS: &[&str] = &[
    "depset",
    "load",
    "package",
    "alias",
    "filegroup",
    "cc_library",
    "test_suite",
    "glob",
    "select",
    "dict",
];
pub const REVIEWED_MACROS: &[(&str, &str)] = &[
    (CAPNP_DEFINITIONS, CAPNP_LIBRARY_MACRO),
    (CAPNP_GENERATION, CAPNP_SOURCES_MACRO),
    (
        "build_system/bazel/rust/adversarial.bzl",
        "adversarial_runtime_targets",
    ),
    (
        "build_system/bazel/rust/unit_test_fixtures.bzl",
        "unit_test_fixture_environment",
    ),
    (
        "build_system/bazel/lifecycle_deadline_contract.bzl",
        "hub_lifecycle_deadline_rust_contract",
    ),
    (
        "build_system/bazel/lifecycle_deadline_contract.bzl",
        "lifecycle_deadline_rust_contract",
    ),
    (
        "build_system/bazel/process_role_contract.bzl",
        "process_role_rust_contract",
    ),
    (
        "build_system/bazel/scm_rights_descriptor_contract.bzl",
        "scm_rights_descriptor_rust_contract",
    ),
    ("build_system/bazel/rust/isolated_test.bzl", "isolated_rust_test"),
    (
        "build_system/bazel/rust/runtime_test_suite.bzl",
        "runtime_test_suite",
    ),
    (
        "build_system/bazel/rust/socket_root_guard_process.bzl",
        "socket_root_guard_process_test",
    ),
    (
        "build_system/bazel/rust/component_test_graph.bzl",
        "component_test_aliases",
    ),
    (
        "build_system/bazel/rust/component_test_graph.bzl",
        "component_test_dependencies",
    ),
    (
        "build_system/bazel/rust/process_test_deps.bzl",
        "process_test_deps",
    ),
    (
        "build_system/bazel/rust/source_paths.bzl",
        "canonical_source_path_flags",
    ),
];
