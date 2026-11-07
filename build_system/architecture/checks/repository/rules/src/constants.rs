pub const BUILD_SYSTEM_DIRECTORY: &str = "build_system";
pub const THIRD_PARTY_DIRECTORY: &str = "third_party";
pub const INTEGRATION_TESTS_DIRECTORY: &str = "integration_tests";
pub const RETIRED_TOP_LEVEL_DIRECTORIES: &[&str] = &["rust", "tools", "tests"];

pub(crate) const BUILD_FILES: &[&str] = &["BUILD", "BUILD.bazel"];
pub(crate) const DIRECTORY_SEPARATOR: char = '/';
pub(crate) const FLAT_COMPONENT_SEPARATOR: char = '_';
pub(crate) const COMPONENT_ROOTS: &[&str] = &[
    "core/crates/agent",
    "core/crates/application",
    "core/crates/conversation",
    "core/crates/distribution",
    "core/crates/hub",
    "core/crates/persistence",
    "core/crates/plugin",
    "core/crates/process",
    "core/crates/presentation",
    "core/crates/runtime",
    "core/crates/transport",
    "core/crates/terminal",
    "clients/tui",
    "build_system/architecture",
];
pub(crate) const RETIRED_AGENT_DIRECTORIES: &[&str] = &[
    "core/crates/agent_contract",
    "core/crates/agent_runtime",
    "core/crates/agent_system",
];
pub(crate) const TEST_DIRECTORIES: &[&str] = &["unit_tests", "integration_tests", "test_support"];
pub(crate) const COMPONENT_LAYOUT_POLICY_MESSAGE: &str =
    "component families must use directory hierarchy instead of flattened module_function paths";
