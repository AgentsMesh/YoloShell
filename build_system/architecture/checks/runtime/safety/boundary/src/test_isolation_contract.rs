use crate::constants::ROOT_IDENTITY_MARKER;

pub const RUNTIME_TESTS: &str = "integration_tests/runtime/BUILD.bazel";
pub const HOST_TESTS: &str = "core/crates/application/host/cli/BUILD.bazel";
pub const REMOTE_TESTS: &str = "core/crates/application/remote/cli/BUILD.bazel";
pub const ISOLATION_RULE: &str = "build_system/bazel/rust/isolated_test.bzl";
pub const ISOLATION_ENTRYPOINT: &str =
    "core/crates/application/socket/guard/test_support/socket_root_guard_main.rs";
pub const ISOLATION_RUNNER: &str = "core/crates/application/socket/guard/test_support/runner.rs";
pub const ISOLATION_WATCHDOG: &str =
    "core/crates/application/socket/guard/test_support/watchdog_server.rs";
pub const ISOLATION_LAUNCHER: &str =
    "core/crates/application/socket/guard/test_support/launcher.rs";
pub const ISOLATION_CUSTODY: &str = "core/crates/application/socket/guard/test_support/custody.rs";
pub const ISOLATION_ROOT: &str = "core/crates/persistence/paths/src/isolated/isolated_test_socket_root.rs";
pub const ISOLATION_CREATE: &str =
    "core/crates/persistence/paths/src/isolated/isolated_test_socket_root_create.rs";
pub const ISOLATION_FINALIZE: &str =
    "core/crates/persistence/paths/src/isolated/isolated_test_socket_root_finalize.rs";
pub const LIFECYCLE_LOCK: &str = "core/crates/persistence/paths/src/socket/namespace/socket_namespace_lock.rs";
pub const COVERAGE_ENVIRONMENT: &str = "build_system/coverage/collector/src/environment.rs";

pub const RULE_MARKERS: &[&str] = &[
    "//core/crates/application/socket/guard:socket_root_guard",
    "ctx.executable._runner",
    "ctx.attr._runner[DefaultInfo].default_runfiles",
    "executable = True",
    "cfg = \"exec\"",
    "coverage_common.instrumented_files_info(",
    "RunEnvironmentInfo(",
    "YOLO_TEST_EXECUTABLE",
    "yolo-test-wrapper-required",
    "@rules_rust//util/collect_coverage",
];
pub const ENTRYPOINT_MARKERS: &[&str] = &["mod socket_root_guard;", "socket_root_guard::run()"];
pub const RUNNER_MARKERS: &[&str] = &[
    "WatchdogClient::spawn()",
    "RootCustody::adopt(",
    "LauncherClient::spawn(",
    "watchdog.track(process_root)",
    "TreeCustody::capture(",
    "watchdog.finish()",
    "root.disarm()",
    "root.finalize()",
];
pub const WATCHDOG_MARKERS: &[&str] = &[
    "ControlCapability::generate()",
    "DiscoveryCustody::adopt_inherited()",
    "RootCustody::create()",
    "TreeCustody::capture(",
    "tree.settle()",
    "root.finalize()",
];
pub const LAUNCHER_MARKERS: &[&str] = &[
    "Command::new(executable)",
    "SOCKET_ROOT_ENVIRONMENT",
    "TEST_FILTER_ENVIRONMENT",
    ".arg(test_filter)",
    ".env_remove(TEST_FILTER_ENVIRONMENT)",
    ".stdin(Stdio::null())",
    "force_unreaped_direct_child(",
];
pub const CUSTODY_MARKERS: &[&str] = &[
    "IsolatedTestSocketRoot::create()",
    "IsolatedTestSocketRoot::adopt(",
    "pub(super) fn disarm(",
    "CapturedProcessTree::capture_with_discovery(",
    "ProcessDiscoveryCapability::adopt_inherited_manager()",
    "watchdog_cleanup::settle(",
    "self.0.as_mut().unwrap().finalize()",
    "self.settle()",
];
pub const ROOT_MARKERS: &[&str] = &[
    "pub fn finalize(&mut self) -> Result<()> { self.finalization_attempted = true; finalize::finalize(self) }",
    "impl Drop for IsolatedTestSocketRoot { fn drop(&mut self) { if !self.finalization_attempted { let _ = self.finalize(); } } }",
];
pub const REPEATED_ROOT_DROP: &str = "impl Drop for RootCustody";
pub const CREATE_MARKERS: &[&str] = &[
    "libc::mkdirat",
    "libc::O_NOFOLLOW",
    "parent_identity",
    ROOT_IDENTITY_MARKER,
    "ObjectIdentity::from_file",
];
pub const FINALIZE_MARKERS: &[&str] = &[
    "SocketNamespaceLifecycleLock::try_acquire",
    "settlement::settle(root)?",
    "create::verify(root)?",
    "require_empty(&root.root)?",
    "create::rebind_current(root)?",
    "rename_exclusive_between",
    "ObjectIdentity::from_file",
    "current == root.root_identity",
    "libc::AT_REMOVEDIR",
    "remove_retired(root)?; drop(lifecycle);",
];
pub const LIFECYCLE_MARKERS: &[&str] = &[
    "root_lock: ProcessFileLock",
    "_turn: ProcessLockTurn",
    "ProcessLockTurn::acquire(identity, SOCKET_LIFECYCLE_LOCK_TIMEOUT)",
    "ProcessLockTurn::try_acquire(root.object_identity()?)",
    "BorrowedFd::borrow_raw(root.descriptor())",
    "ProcessFileLock::try_lock(&root_file, root_identity, Mode::Exclusive)",
    "root.verify_path()?",
    "root.object_identity()? == self.root_identity",
    "self.root_lock.holds_exclusive(self.root_identity)",
];
pub const COLLECTOR_MARKERS: &[&str] = &[
    "INSTRUMENTED_TEST_BINARY_ENV",
    "select_test_binary(instrumented.as_deref(), bazel.as_deref())",
];
