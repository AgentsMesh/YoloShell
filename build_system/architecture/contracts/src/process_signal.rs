pub use crate::process_signal_retained::RETAINED_EXACT_DOUBLE_FORK_TESTS_PATH;

pub const LIVE_CONTROL_PATH: &str =
    "core/crates/runtime/engine/process/src/live/live_identity_control.rs";
pub const SIGNAL_AUTHORITY_PATH: &str =
    "core/crates/runtime/engine/process/src/signal/signal_authority.rs";
pub const SIGNAL_AUTHORITY_EXACT_PATH: &str =
    "core/crates/runtime/engine/process/src/signal/signal_authority_exact.rs";
pub const PIDFD_PATH: &str = "core/crates/runtime/engine/process/src/live/live_identity/linux_pidfd.rs";
pub const PIDFD_KERNEL_PATH: &str =
    "core/crates/runtime/engine/process/src/live/live_identity/linux_pidfd/kernel.rs";
pub const MACOS_SIGNAL_PATH: &str = "core/crates/runtime/engine/process/src/macos/macos_signal.rs";
pub const BOOTSTRAP_TRACE_PATH: &str =
    "core/crates/runtime/engine/worker/src/bootstrap/bootstrap_exec_observer_linux_trace.rs";
pub const BOOTSTRAP_OBSERVER_PATH: &str =
    "core/crates/runtime/engine/worker/src/bootstrap/bootstrap_exec_observer.rs";
pub const INVENTORY_PATH: &str = "core/crates/runtime/engine/process/src/inventory/inventory.rs";
pub const INVENTORY_MEMBER_SIGNAL_PATH: &str =
    "core/crates/runtime/engine/process/src/inventory/inventory_member_signal.rs";
pub const SIGNAL_PROCESS_GROUP_MEMBER: &str = "signal_process_group_member";
pub const MEMBER_SIGNAL_TEST_SUPPORT_PATH: &str =
    "core/crates/runtime/engine/process/test_support/inventory/inventory_member_signal.rs";
pub const MEMBER_SIGNAL_UNIT_TEST_PATH: &str =
    "core/crates/runtime/engine/process/unit_tests/inventory_member_signal_tests.rs";
pub const INVENTORY_SCOPE_PATH: &str = "core/crates/runtime/engine/process/src/inventory/inventory_scope.rs";
pub const CAPTURED_TARGETS_PATH: &str =
    "core/crates/runtime/engine/process/src/captured/captured_targets.rs";
pub const CAPTURED_TARGETS_SIGKILL_PATH: &str =
    "core/crates/runtime/engine/process/src/captured/captured_targets_sigkill.rs";
pub const CAPTURED_TREE_PATH: &str = "core/crates/runtime/engine/process/src/captured/captured_tree.rs";
pub const EXACT_TERMINATION_PATH: &str =
    "core/crates/runtime/engine/process/src/exact/exact_termination.rs";
pub const REMOTE_SUPERVISED_CONTAINMENT_PATH: &str =
    "core/crates/transport/remote/broker/src/supervision/supervision_containment.rs";
pub const BROKER_PENDING_PATH: &str = "core/crates/application/broker/child/custody/src/pending.rs";
pub const BROKER_SETTLEMENT_PATH: &str =
    "core/crates/application/broker/child/custody/src/spawned_settlement.rs";
pub const BROKER_REAPER_CUSTODY_PATH: &str =
    "core/crates/application/broker/child/custody/src/spawned_termination_custody.rs";
pub const BROKER_REAPER_WORKER_PATH: &str =
    "core/crates/application/broker/child/custody/src/spawned_termination_worker.rs";
pub const COMMAND_SETTLEMENT_PATH: &str =
    "core/crates/runtime/engine/process/src/command/command_settlement.rs";
pub const CHILD_REAPER_WORKER_PATH: &str =
    "core/crates/runtime/engine/process/src/child/child_reaper_worker.rs";
pub const BRIDGE_CHILD_PATH: &str = "core/crates/application/host/cli/src/bridge_child.rs";
pub const SOCKET_ROOT_DIRECT_CHILD_PATH: &str =
    "core/crates/application/socket/guard/test_support/direct_child.rs";
pub const SOCKET_ROOT_PROCESS_TEST_SUPPORT_PATH: &str =
    "core/crates/application/socket/guard/process/integration_tests/process_support.rs";
pub const DETACHED_CHILD_CUSTODY_PATH: &str =
    "core/crates/runtime/engine/process/src/detached_child_custody.rs";
pub const PTY_SESSION_LINUX_INTEGRATION_HOLDER_PATH: &str =
    "core/crates/runtime/engine/process/src/pty_session_linux_integration_holder.rs";
pub const PTY_SESSION_LINUX_INTEGRATION_HOLDER_FILE: &str =
    "pty_session_linux_integration_holder.rs";
pub const RAW_SIGNAL_MESSAGE: &str =
    "raw process signal primitive is outside its exact file and function owner";
pub const PIDFD_SIGNAL_MESSAGE: &str =
    "pidfd signal syscall is outside the retained kernel pidfd sender";
pub const AUDIT_SIGNAL_MESSAGE: &str =
    "macOS audit-token signal primitive is outside its exact wrapper";
pub const AUTHORIZED_ENTRY_MESSAGE: &str =
    "exact process signal entry is outside its private authority owner";
pub const AUTHORITY_TYPE_MESSAGE: &str =
    "process signal authority type escaped its approved ownership modules";
pub const AUTHORITY_CONSTRUCTOR_MESSAGE: &str =
    "process signal authority constructor moved outside its verified owner";
pub const BRIDGE_CFG_MESSAGE: &str =
    "test signal bridge requires cfg(test) or the adversarial-test-hooks feature";
pub const BRIDGE_OWNER_MESSAGE: &str = "test signal bridge moved outside its exact owner";
pub const CHILD_KILL_MESSAGE: &str =
    "direct child termination is outside its exact process custody owner";
pub const CHILD_FLOW_MESSAGE: &str = "direct child termination is not fenced before its exact wait";
pub const DIRECT_CUSTODY_MESSAGE: &str =
    "direct child termination request escaped its exact retained custody call graph";
pub const CHILD_LIFECYCLE_MESSAGE: &str =
    "implicit child termination is forbidden outside explicit custody";
pub const SIGNAL_EXECUTABLE_MESSAGE: &str =
    "process signal executable literal is forbidden in production Rust";
pub const TEST_MODULE_MESSAGE: &str =
    "test-named Rust modules require a production-excluding cfg(test) guard";
pub const LIVE_IDENTITY: &str = "LiveProcessIdentity";
pub const PROCESS_IDENTITY: &str = "ProcessIdentity";
pub const PROCESS_SIGNAL_AUTHORITY: &str = "ProcessSignalAuthority";
pub const DESTRUCTIVE_ACCESS: &str = "DestructiveSessionAccess";
pub const DESTRUCTIVE_AUTHORITY: &str = "DestructiveSessionAuthority";
pub const FOREGROUND_AUTHORITY: &str = "ForegroundSignalAuthority";
pub const PROCESS_GROUP_IDENTITY: &str = "ProcessGroupIdentity";
pub const RETAINED_EXACT_PROCESS: &str = "RetainedExactProcess";
pub const PIDFD_OPERATIONS: &str = "KernelPidfdOperations";
pub const PIDFD_OPEN_OWNER: &str = "open";
pub const PTY_SCOPE: &str = "PtyProcessScope";
pub const FORKED_EXEC: &str = "ForkedExec";
pub const CHILD: &str = "Child";
pub const DETACHED_CHILD_CUSTODY: &str = "DetachedChildCustody";
pub const SPAWNED_HOST: &str = "SpawnedHost";
pub const SECCOMP_OWNER_PATH: &str =
    "core/crates/runtime/engine/process/seccomp/src/lib.rs";
pub const SECCOMP_OWNER_FUNCTION: &str = "install_aarch64";
pub const SECCOMP_PRCTL_OPTIONS: &[&str] = &["PR_SET_NO_NEW_PRIVS", "PR_SET_SECCOMP"];

pub use crate::process_signal_retained::vocabulary::*;
