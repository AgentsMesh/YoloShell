pub const CORE_PREFIX: &str = "core/";
pub const CRATE_ROOT_ATTRIBUTE: &str = "crate_root";
pub const NAME_ATTRIBUTE: &str = "name";
pub const RUST_BINARY_RULE: &str = "rust_binary";
pub const RUST_LIBRARY_RULE: &str = "rust_library";
pub const RUST_STATIC_LIBRARY_RULE: &str = "rust_static_library";
pub const RUST_TEST_RULE: &str = "rust_test";
pub const SOURCES_ATTRIBUTE: &str = "srcs";
pub const TEST_ONLY_ATTRIBUTE: &str = "testonly";
pub const TRUE_LITERAL: &str = "True";
pub const PACKAGE: &str = "core/crates/application/yolo";
pub const LAUNCH_PACKAGE: &str = "core/crates/application/launch/contract";
pub const PURE_LAUNCH_POLICY: &str = "launch_contract is a std-only boundary and cannot depend on runtime, business or external libraries";
pub const BUILD: &str = "core/crates/application/yolo/BUILD.bazel";
pub const NAME: &str = "yolo";
pub const ROOT: &str = "src/main.rs";
pub const SOURCE_PREFIX: &str = "core/crates/application/yolo/src/";
pub const REQUIRED_EXECUTABLES: usize = 1;
pub const FALSE_LITERAL: &str = "False";
pub const MAX_SOURCES: usize = 4;
pub const MAX_LINES: usize = 160;
pub const DEPENDENCIES_ATTRIBUTE: &str = "deps";
pub const EXECUTABLE_POLICY: &str = "core must declare exactly one production Rust executable: //core/crates/application/yolo:yolo; other binaries require literal testonly = True";
pub const COMPOSITION_POLICY: &str = "yolo composition root requires src/main.rs and statically resolved local sources within its small source and line budgets";
pub const DEPENDENCY_POLICY: &str = "yolo composition root dependencies must resolve only to role entry libraries, launch_contract, anyhow and clap";
pub const RULE_RESOLUTION_POLICY: &str =
    "core executable rules must resolve unambiguously to rules_rust";
pub const CALLABLE_POLICY: &str = "core BUILD calls must use direct rules or a reviewed source-and-symbol macro; callable aliases and unreviewed wrappers can hide production binaries";
pub use yolo_architecture_policy_contracts::entrypoint_macros::*;
pub const ALLOWED_DEPENDENCIES: &[&str] = &[
    "//plugins/web/gateway/cli:yolo_web_cli",
    "//plugins/catalog:yolo_builtin_catalog",
    "//core/crates/agent/cli:yolo_agent_cli",
    "//core/crates/conversation/cli:yolo_conversation_cli",
    "//plugins/discovery:yolo_protocol_cli",
    "//core/crates/plugin/client/host:yolo_builtin_client_host",
    "//core/crates/application/launch/contract:yolo_launch_contract",
    "//core/crates/application/host/cli:yolo_host_cli",
    "//core/crates/application/attach/cli:yolo_attach_cli",
    "//core/crates/application/remote/cli:yolo_remote_cli",
    "//core/crates/application/upgrade/cli:yolo_upgrade_cli",
    "//core/crates/application/workspace/cli:yolo_workspace_cli",
    "//core/crates/application/trusted-plugin/cli:yolo_trusted_plugin_cli",
    "//core/crates/hub/cli:yolo_hub_cli",
    "//core/crates/hub/attach/cli:yolo_hub_attach_cli",
    "//core/crates/hub/file/helper:yolo_hub_file_helper",
    "//plugins/tui/cli:yolo_tui_cli",
    "@crates//:anyhow",
    "@crates//:clap",
];
