pub const CARGO_EXECUTABLE: &str = "cargo";
pub const CARGO_MANIFEST_PATH: &str = "Cargo.toml";
pub const CARGO_PACKAGE_TABLE: &str = "package";
pub const CARGO_LIBRARY_TABLE: &str = "lib";
pub const CARGO_FALSE_LITERAL: &str = "false";
pub const CARGO_FORBIDDEN_TABLES: &[&str] =
    &["bench", "bin", "example", "profile", "test", "workspace"];
pub const CARGO_FORBIDDEN_TABLE_PREFIXES: &[&str] = &["profile.", "workspace."];
pub const CARGO_REQUIRED_PACKAGE_FALSE_KEYS: &[&str] = &[
    "autobenches",
    "autobins",
    "autoexamples",
    "autotests",
    "build",
    "publish",
];
pub const CARGO_REQUIRED_LIBRARY_FALSE_KEYS: &[&str] = &["bench", "doctest", "test"];
pub const CARGO_REQUIRED_LIBRARY_STRINGS: &[(&str, &str)] = &[
    ("name", "yolo_dependency_metadata"),
    ("path", "build_system/bazel/cargo_metadata/src/lib.rs"),
];
pub const CARGO_PRODUCTION_TARGET_MESSAGE: &str =
    "Cargo manifest must not expose production targets, profiles, or workspaces";
pub const CARGO_METADATA_SHAPE_MESSAGE: &str =
    "Cargo manifest must expose only the fixed dependency-metadata library";
pub const CARGO_DECLARATION_COMMANDS: &[&str] = &["export", "local", "readonly", "typeset"];
pub const CARGO_COMMAND_PREFIXES: &[&str] = &[
    POLICY_COMMAND,
    POLICY_ENV,
    POLICY_EVAL,
    POLICY_EXEC,
    POLICY_NICE,
    POLICY_NOHUP,
    POLICY_SUDO,
    POLICY_XCRUN,
];
pub const SHELL_WRAPPERS: &[&str] = &["bash", "sh", "zsh"];
pub const STARLARK_COMMAND_ATTRIBUTES: &[&str] = &[
    "cmd", "cmd_bash", "cmd_bat", "cmd_ps", "command", "commands",
];
pub const STARLARK_EXTENSIONS: &[&str] = &["bazel", "bzl"];
pub const CONTROL_WORDS: &[&str] = &["if", "then", "elif", "else", "while", "until", "do", "!"];
pub const TIME_COMMAND: &str = "time";
pub const TIME_VALUE_OPTIONS: &[&str] = &["-f", "--format", "-o", "--output"];
pub const COMMAND_QUERY_OPTIONS: &[&str] = &["-v", "-V", "--help"];
pub const ENV_VALUE_OPTIONS: &[&str] = &["-u", "--unset", "-C", "--chdir", "-a", "--argv0"];
pub const ENV_SPLIT_OPTIONS: &[&str] = &["-S", "--split-string"];
pub const NON_EXECUTING_OPTIONS: &[&str] = &["--help", "--version"];
pub const EXEC_VALUE_OPTIONS: &[&str] = &["-a"];
pub const NICE_VALUE_OPTIONS: &[&str] = &["-n", "--adjustment"];
pub const SUDO_VALUE_OPTIONS: &[&str] = &[
    "-C",
    "--close-from",
    "-D",
    "--chdir",
    "-g",
    "--group",
    "-h",
    "--host",
    "-p",
    "--prompt",
    "-R",
    "--chroot",
    "-r",
    "--role",
    "-t",
    "--type",
    "-u",
    "--user",
];
pub const SUDO_QUERY_OPTIONS: &[&str] = &["-V", "--help", "--version"];
pub const XCRUN_VALUE_OPTIONS: &[&str] = &["--sdk", "--toolchain"];
pub const XCRUN_QUERY_OPTIONS: &[&str] = &[
    "-f",
    "--find",
    "--show-sdk-build-version",
    "--show-sdk-path",
    "--show-sdk-platform-path",
    "--show-sdk-platform-version",
    "--show-sdk-version",
];
pub const OPTION_END: &str = "--";
pub const ENV_SPLIT_PREFIX: &str = "--split-string=";
pub const POLICY_COMMAND: &str = "command";
pub const POLICY_ENV: &str = "env";
pub const POLICY_EVAL: &str = "eval";
pub const POLICY_EXEC: &str = "exec";
pub const POLICY_NICE: &str = "nice";
pub const POLICY_NOHUP: &str = "nohup";
pub const POLICY_SUDO: &str = "sudo";
pub const POLICY_XCRUN: &str = "xcrun";
pub const MACOS_TEST_RULE: &str = "macos_unit_test";
pub const RELEASE_SUITE_COVERAGE_TARGETS_PATH: &str = "build_system/platform/macos/coverage_targets.bzl";
pub const RELEASE_SUITE_ROOT_BUILD_PATH: &str = "BUILD.bazel";
pub const RELEASE_SUITE_RULE: &str = "test_suite";
pub const RELEASE_SUITE_NATIVE_RECEIVER: &str = "native";
pub const RELEASE_SUITE_COVERAGE_TEST_RULE: &str = "sh_test";
pub const RELEASE_SUITE_NAME_ATTRIBUTE: &str = "name";
pub const RELEASE_SUITE_TESTS_ATTRIBUTE: &str = "tests";
pub const RELEASE_SUITE_UNIT_NAME: &str = "unit";
pub const RELEASE_SUITE_RELEASE_UNIT_NAME: &str = "release_unit";
pub const RELEASE_SUITE_UNIT_REFERENCE: &str = ":unit";
pub const RELEASE_SUITE_RELEASE_TESTS_REFERENCE: &str = "//build_system/release:tests";
pub const RELEASE_SUITE_RELEASE_E2E_NAME: &str = "release_e2e";
pub const RELEASE_SUITE_VISIBLE_E2E_NAME: &str = "visible_e2e";
pub const RELEASE_SUITE_MACOS_TARGET_PREFIX: &str = "//plugins/macos:";
pub const RELEASE_SUITE_POLICY_MESSAGE: &str =
    "release unit gate must include every Swift coverage policy regression";
pub const RELEASE_SUITE_FOCUS_POLICY_MESSAGE: &str =
    "release E2E gate must not include visible-window tests";
pub const FRAME_RANGE_CALLS: &[&str] = &["drain", "get"];
pub const SOCKET_ROOT_BOUNDARY_WRITER: &str = "atomic_write_entry_guarded_with_boundary";
pub const SOCKET_ROOT_ENTRY_WRITER: &str = "atomic_write_entry_guarded";
pub const SOCKET_ROOT_PATH_WRITER: &str = "atomic_write_json";
pub const SESSION_MARKER_CAS_WRITER: &str = "replace_entry_guarded";
pub const SESSION_MARKER_CAS_OWNER_FUNCTION: &str = "replace_resume_binding_marker";
pub const CODEX_AUTHORITY_SEND_REPLACE: &str = "send_replace";
pub const CODEX_AUTHORITY_SEND_IF_MODIFIED: &str = "send_if_modified";
pub const CODEX_AUTHORITY_PUBLISH_PROVEN: &str = "publish_proven";
pub const CODEX_AUTHORITY_PUBLISH_LOST: &str = "publish_authority_lost";
pub const CODEX_AUTHORITY_PUBLISH_STORAGE: &str = "publish_storage_unavailable";
pub const CODEX_AUTHORITY_PUBLISH_INITIAL: &str = "publish_initial";
pub const CODEX_AUTHORITY_PUBLISH_SUPERSEDED: &str = "publish_superseded";
pub const CODEX_AUTHORITY_SUPERSEDE_LOCKED: &str = "supersede_locked";
pub const CODEX_AUTHORITY_CONTAIN_FAILED_TAKEOVER: &str = "contain_failed_takeover";
pub const CODEX_AUTHORITY_PUBLISH_LIFECYCLE: &str = "publish_lifecycle";
pub const CODEX_AUTHORITY_PUBLISH_ACTIVE: &str = "publish_active";
pub const CODEX_AUTHORITY_PUBLISH_TERMINAL_LOST: &str = "publish_terminal_authority_lost";
pub const CODEX_AUTHORITY_PUBLISH_TERMINAL_STORAGE: &str = "publish_terminal_storage_unavailable";
pub const CODEX_AUTHORITY_CONTROL_OWNER: &str = "CodexAuthorityControl";
pub const CODEX_AUTHORITY_CONSTRUCTOR: &str = "new";
pub const CODEX_AUTHORITY_TAKEOVER_BINDING: &str = "takeover";
pub const CODEX_AUTHORITY_INSTALL_METHOD: &str = "install";
pub const PTY_ALIAS_BLOCKING_LOCK_OPERATION: &str = "blocking_lock_owned";
pub const PTY_ALIAS_LOCK_OPERATION: &str = "lock_owned";
pub const PTY_ALIAS_BLOCK_ON_OPERATION: &str = "block_on";
pub const PTY_ALIAS_YIELD_OPERATION: &str = "yield_now";
pub const PTY_ALIAS_TRY_LOCK_OPERATION: &str = "try_lock_owned";
pub const PTY_ALIAS_BLOCKING_ADMISSION: &str = "admit_system_mutation_blocking";
pub const PTY_ALIAS_TERMINATION_FUNCTION: &str = "terminate_pty_processes";
pub const PTY_ALIAS_TERMINATION_PERMIT: &str = "PtyTerminationPermit";
pub const PTY_ALIAS_PERMIT_BINDING: &str = "permit";
pub const PTY_ALIAS_OWNER_SUBMISSION: &str = "submit_termination";
pub const PTY_ALIAS_ADMITTED_CLOSE: &str = "close_runtime_pty_aliases_admitted";
pub const PTY_ALIAS_POLL_CLOSE: &str = "poll_runtime_pty_alias_closure";
pub const PTY_ALIAS_TASK_ADMITTED_CLOSE: &str = "close_runtime_pty_aliases_task_admitted";
pub const PTY_ALIAS_TASK_PERMIT: &str = "AliasClosureMutationPermit";
pub const PTY_ALIAS_SYSTEM_PERMIT: &str = "SystemMutationPermit";
pub const PTY_ALIAS_SYSTEM_AUTHORITY_FIELD: &str = "system_release_authorized";
pub const PTY_ALIAS_SYSTEM_AUTHORITY_QUERY: &str = "system_release_is_authorized";
pub const PTY_ALIAS_SYSTEM_ADMITTED_CLOSE: &str = "close_runtime_pty_aliases_system_admitted";
pub const PTY_ALIAS_SYSTEM_AUTHORIZED_CLOSE: &str = "close_runtime_pty_aliases_system_authorized";
