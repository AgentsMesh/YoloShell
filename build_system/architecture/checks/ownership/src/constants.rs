pub const BUILD_FILE_NAMES: &[&str] = &["BUILD", "BUILD.bazel"];
pub const OWNERSHIP_POLICY_BUILD_FILE: &str =
    "build_system/architecture/checks/ownership/BUILD.bazel";
pub const BUILD_SYSTEM_DIRECTORY: &str = "build_system";
pub const BUILD_SYSTEM_PREFIX: &str = "build_system/";
pub const CORE_PREFIX: &str = "core/";
pub const CLIENTS_PREFIX: &str = "clients/";
pub const INTEGRATION_TESTS_PREFIX: &str = "integration_tests/";
pub const RUST_SOURCE_EXTENSION: &str = "rs";
pub const SOURCE_DIRECTORY: &str = "src";
pub const STARLARK_EXTENSION: &str = "bzl";
pub const UNIT_TEST_DIRECTORY: &str = "unit_tests";
pub const INTEGRATION_TEST_DIRECTORY: &str = "integration_tests";
pub const TEST_SUPPORT_DIRECTORY: &str = "test_support";
pub const TEST_SUPPORT_PACKAGE_SUFFIX: &str = "_test_support";
pub const FIXTURE_DIRECTORY: &str = "fixtures";
pub const TEST_DIRECTORY_SUFFIX: &str = "_tests";
pub const RUST_LIBRARY_RULE: &str = "rust_library";
pub const RUST_STATIC_LIBRARY_RULE: &str = "rust_static_library";
pub const RUST_BINARY_RULE: &str = "rust_binary";
pub const RUST_TEST_RULE: &str = "rust_test";
pub const ISOLATED_RUST_TEST_RULE: &str = "isolated_rust_test";
pub const FILEGROUP_RULE: &str = "filegroup";
pub const ALIAS_RULE: &str = "alias";
pub const GLOB_FUNCTION: &str = "glob";
pub const LOAD_FUNCTION: &str = "load";
pub const STARLARK_CONTROL_KEYWORDS: &[&str] =
    &["if", "elif", "for", "in", "and", "or", "not", "return"];
pub const NAME_ATTRIBUTE: &str = "name";
pub const SOURCES_ATTRIBUTE: &str = "srcs";
pub const ACTUAL_ATTRIBUTE: &str = "actual";
pub const EXCLUDE_ATTRIBUTE: &str = "exclude";
pub const TEST_ONLY_ATTRIBUTE: &str = "testonly";
pub const TRUE_LITERAL: &str = "True";
pub const CRATE_ROOT_ATTRIBUTE: &str = "crate_root";
pub const CRATE_FEATURES_ATTRIBUTE: &str = "crate_features";
pub const PATH_ATTRIBUTE_NAME: &str = "path";
pub const RUNTIME_SOURCES_TARGET: &str = "runtime_sources";
pub const SOURCE_GROUP_SUFFIX: &str = "_sources";
pub const GENERATED_DIRECTORY: &str = "generated";
// These dual budgets keep each build-time crate below one focused review unit.
pub const MAX_BUILD_SYSTEM_PRODUCTION_SOURCES_PER_TARGET: usize = 24;
pub const MAX_BUILD_SYSTEM_PRODUCTION_SOURCES_PER_PACKAGE: usize = 24;
pub const MAX_BUILD_SYSTEM_PRODUCTION_LINES_PER_PACKAGE: usize = 2_400;
pub const MAX_BUILD_SYSTEM_TEST_SOURCES_PER_TARGET: usize = 24;
// Core and client packages share the same reviewable unit; existing packages
// above it are recorded in PACKAGE_BASELINE below as grow-only ceilings.
// Include-composition and dependency-usage baselines instead live in .bzl
// tuples, use the shared strict reader, and also fail when an entry is stale.
pub const MAX_CORE_PRODUCTION_SOURCES_PER_PACKAGE: usize = 24;
pub const MAX_CORE_PRODUCTION_LINES_PER_PACKAGE: usize = 2_400;

pub const MISSING_OWNER_MESSAGE: &str = "production Rust source must belong to exactly one directly declared Rust library, static library or binary";
pub const DUPLICATE_OWNER_MESSAGE: &str =
    "production Rust source is compiled by more than one Rust library, static library or binary";
pub const CROSS_PACKAGE_AGGREGATION_MESSAGE: &str =
    "production Rust sources cannot cross package boundaries through filegroups";
pub const CROSS_CRATE_PATH_MESSAGE: &str =
    "production #[path] cannot cross a Rust crate package boundary";
pub const SOURCE_BUDGET_MESSAGE: &str =
    "build-system Rust target exceeds the production source budget";
pub const PACKAGE_SOURCE_BUDGET_MESSAGE: &str =
    "build-system Bazel package exceeds the production Rust source budget";
pub const TEST_SOURCE_BUDGET_MESSAGE: &str =
    "build-system Rust test target exceeds the test-owned source budget";
pub const PACKAGE_LINE_BUDGET_MESSAGE: &str =
    "build-system Bazel package exceeds the production Rust line budget";
pub const CORE_PACKAGE_SOURCE_BUDGET_MESSAGE: &str =
    "core or client Bazel package exceeds the production Rust source budget";
pub const CORE_PACKAGE_LINE_BUDGET_MESSAGE: &str =
    "core or client Bazel package exceeds the production Rust line budget";
pub const STALE_PACKAGE_BASELINE_MESSAGE: &str =
    "package baseline entry no longer matches a production package";
pub const UNRESOLVED_SOURCE_EXPRESSION_MESSAGE: &str =
    "build-system Rust target source expression must be statically resolvable";
pub const MISSING_TEST_OWNER_MESSAGE: &str =
    "Rust test or support source must belong to at least one local test target";
pub const FACADE_OWNERSHIP_MESSAGE: &str =
    "root facade cannot compile sources owned by a component package";
pub const CROSS_PACKAGE_TEST_OWNERSHIP_MESSAGE: &str =
    "Rust test targets cannot compile sources owned by another Bazel package";
pub const MIXED_TEST_CLASS_MESSAGE: &str =
    "unit and integration test sources require separate Rust test targets";
pub const PRODUCTION_TEST_SOURCE_MESSAGE: &str =
    "build-system production Rust targets cannot compile test-owned sources";
pub const PRODUCTION_SOURCE_LAYOUT_MESSAGE: &str =
    "build-system production Rust source must live in its owning package's src directory";
pub const UNREACHABLE_TEST_MODULE_MESSAGE: &str =
    "Rust test source is not reachable from a declared test crate root with its test features";
pub const UNRESOLVED_TEST_ROOT_MESSAGE: &str =
    "Rust test crate root must resolve to one source included in its srcs";
pub const UNCERTAIN_TEST_MODULE_MESSAGE: &str =
    "Rust test module reachability is unresolved by the static module graph";
// Grandfathered package sizes as (package, production sources, production
// lines). Growth past either recorded value fails the gate.
pub const PACKAGE_BASELINE: &[(&str, usize, usize)] = &[
    ("core/crates/agent/providers/claude", 61, 6374),
    ("core/crates/agent/providers/codex/hooks", 97, 8383),
    ("core/crates/agent/system/hooks/runtime", 39, 3077),
    ("core/crates/hub/federation", 24, 2439),
    ("core/crates/hub/local/backend", 41, 3297),
    ("core/crates/persistence/catalog", 49, 5234),
    ("core/crates/persistence/durable/reboot", 31, 3575),
    ("core/crates/persistence/journal", 27, 2910),
    ("core/crates/persistence/paths", 177, 16407),
    ("core/crates/runtime/engine/custody/guardian", 47, 4711),
    ("core/crates/runtime/engine/host", 234, 22405),
    ("core/crates/runtime/engine/process", 285, 28999),
    ("core/crates/runtime/engine/pty/custody", 28, 0),
    ("core/crates/runtime/engine/worker", 469, 43002),
    ("core/crates/transport/relay", 33, 2910),
    ("core/crates/transport/remote", 79, 8027),
    ("core/crates/transport/remote/broker", 99, 9083),
    ("clients/tui/app", 150, 13405),
];
