pub const PTY_SESSION_SOURCE_PREFIXES: &[&str] = &[
    "core/crates/runtime/engine/process/src/pty_session",
    "core/crates/runtime/engine/process/unit_tests/pty_session",
    "core/crates/runtime/engine/process/integration_tests/pty_session",
];
pub const BINARY_SOURCE_PREFIXES: &[&str] = &[
    "core/crates/application/broker/child/custody/",
    "core/crates/application/broker/profile/authority/",
    "core/crates/application/broker/startup/",
    "core/crates/application/broker/supervision/",
    "core/crates/application/broker/supervision/state/",
    "core/crates/application/attach/cli/",
    "core/crates/application/host/cli/",
    "core/crates/application/remote/cli/",
    "core/crates/application/yolo/",
    "core/crates/hub/cli/",
    "plugins/tui/cli/",
    "core/crates/application/socket/guard/",
    "core/crates/application/socket/guard/process/",
];
pub const BINARY_SOURCE_FILES: &[&str] = &[
    "core/crates/application/attach/cli/src/lib.rs",
    "core/crates/application/host/cli/src/lib.rs",
    "core/crates/application/remote/cli/src/lib.rs",
    "core/crates/application/yolo/src/main.rs",
    "core/crates/hub/cli/src/lib.rs",
    "plugins/tui/cli/src/lib.rs",
    "core/crates/application/socket/guard/test_support/socket_root_guard_main.rs",
    "core/crates/application/socket/guard/process/integration_tests/socket_root_guard_process_tests.rs",
];
pub const COMMAND_TYPE: &str = "Command";
pub const COMMAND_FACTORY_SUFFIX: &str =
    yolo_architecture_policy_contracts::constants::POLICY_COMMAND;
pub const SPAWN_METHOD: &str = "spawn";
pub const SPAWN_GUARDS: &[&str] = &[
    "waitable_child_spawn_guard",
    "waitable_child_spawn_guard_before",
];
pub const SPAWN_BOUNDARY_TYPE: &str = "WaitableChildSpawnBoundary";
pub const SPAWN_BOUNDARY_SOURCE: &str = "core/crates/process/spawn/src/spawn_gate.rs";
pub const POLICY_MESSAGE: &str =
    "runtime test child creation must enter the waitable spawn gate before Command::spawn";
