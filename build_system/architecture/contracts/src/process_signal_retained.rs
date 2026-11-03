pub use crate::build_constants::{
    CFG_ATTR_ATTRIBUTE, CRATE_NAME_ATTRIBUTE, CRATE_ROOT_ATTRIBUTE, NAME_ATTRIBUTE,
    REPOSITORY_NAME_ATTRIBUTE, RUST_BINARY_RULE, RUST_LIBRARY_RULE, RUST_PROC_MACRO_RULE,
    RUST_SHARED_LIBRARY_RULE, RUST_STATIC_LIBRARY_RULE, RUST_TEST_RULE, TESTONLY_ATTRIBUTE,
};

pub const MACRO_RULES: &str = "macro_rules";
pub const PTY_PROOF_WITHOUT_OBJECT_LEASE_FUNCTION: &str =
    "pty_proof_without_object_lease_never_signals";
pub const PTY_EXACT_OBSERVER_REFRESH_FAILURE_FUNCTION: &str =
    "pty_exact_observer_ignores_injected_refresh_failure";
pub const LATE_ESCAPED_DESCENDANT_FUNCTION: &str =
    "root_exit_with_a_late_escaped_descendant_is_indeterminate";
pub const ASSERT_REFRESH_FAILURE_FUNCTION: &str = "assert_refresh_failure";
pub const ENFORCE_EXACT_TEST_CLEANUP_FUNCTION: &str = "enforce_exact_test_cleanup";
pub const PROCESS_MODULE_PATH: &str = "core/crates/runtime/engine/process/src/mod.rs";
pub const RETAINED_EXACT_DOUBLE_FORK_TESTS_PATH: &str =
    "core/crates/runtime/engine/process/integration_tests/retained/retained_exact_double_fork_tests.rs";
pub const NAMESPACE_LOSS_CONTAINMENT_OWNER: &str = "NamespaceLossContainment";
pub const CONTAINMENT_FUNCTION: &str = "containment";
pub const TARGET_FUNCTION: &str = "target";
pub const CAPTURE_PTY_FUNCTION: &str = "capture_pty";
pub const CAPTURE_PTY_FOR_TEST_FUNCTION: &str = "capture_pty_for_test";
pub const CAPTURE_PTY_EXCLUDING_FUNCTION: &str = "capture_pty_excluding";
pub const CAPTURE_WITH_FUNCTION: &str = "capture_with";
pub const DESTRUCTIVE_FUNCTION: &str = "destructive";
pub const EXACT_OBSERVER_FUNCTION: &str = "exact_observer";
pub const TREE_FUNCTION: &str = "tree";
pub const TREE_MUT_FUNCTION: &str = "tree_mut";
pub const FAIL_NEXT_REFRESH_FUNCTION: &str = "fail_next_refresh_for_test";
pub const IS_EXACT_OBSERVER_FOR_TEST_FUNCTION: &str = "is_exact_observer_for_test";
pub const STATE_SNAPSHOT_FOR_TEST_FUNCTION: &str = "state_snapshot_for_test";
pub const UNSUPPORTED_IDENTIFIER_MESSAGE: &str =
    "non-ASCII Rust identifiers are unsupported by retained authority analysis";
pub const UNCLASSIFIED_METHOD_MESSAGE: &str =
    "retained authority method is not classified by architecture policy";
pub const RESOLUTION_INDETERMINATE_MESSAGE: &str = "retained authority resolution is indeterminate";
pub const BRIDGE_PATH_COMPONENT: &str = "bridge";

pub mod vocabulary {
    pub const SIGNAL: &str = "signal";
    pub const SIGNAL_AUTHORIZED: &str = "signal_authorized";
    pub const SIGNAL_FOR_TEST: &str = "signal_for_test";
    pub const AUTHORIZE_FOR_TEST: &str = "authorize_for_test";
    pub const INSTALL_LINUX_CONTAINMENT: &str = "install_linux_containment";
    pub const OPERATION: &str = "operation";
    pub const FOR_TEST: &str = "for_test";
    pub const NEW: &str = "new";
    pub const VERIFIED: &str = "verified";
    pub const CAPTURE: &str = "capture";
    pub const IS_RUNNING: &str = "is_running";
    pub const BEGIN_CONTAINMENT: &str = "begin_containment";
    pub const ENFORCE_CONTAINMENT: &str = "enforce_containment";
    pub const SETTLE: &str = "settle";
    pub const REQUEST_WAITABLE_ROOT_GRACEFUL_TERMINATION: &str =
        "request_waitable_root_graceful_termination";
    pub const SIGNAL_EXACT_PROCESS: &str = "signal_exact_process";
    pub const FORCE_UNREAPED_DIRECT_CHILD: &str = "force_unreaped_direct_child";
    pub const SIGNAL_GUARD_GROUP: &str = "signal_guard_group";
    pub const REQUEST_DIRECT_TERMINATION: &str = "request_direct_termination";
    pub const REQUEST_TERMINATION: &str = "request_termination";
    pub const TERMINATE_WITHOUT_EXACT_CUSTODY: &str = "terminate_without_exact_custody";
    pub const TERMINATE_AND_WAIT_STATUS: &str = "terminate_and_wait_status";
    pub const DROP: &str = "drop";
    pub const THREAD_BELONGS_TO_PROCESS: &str = "thread_belongs_to_process";
    pub const TERMINATE: &str = "terminate";
    pub const POLL_EXITED: &str = "poll_exited";
    pub const REAP: &str = "reap";
    pub const SETTLE_FAILED_CHILD: &str = "settle_failed_child";
    pub const COMPENSATE_HANDOFF_FAILURE: &str = "compensate_handoff_failure";
    pub const HANDOFF_DETACHED_CHILD_CUSTODY: &str = "handoff_detached_child_custody";
    pub const PROCESS_IS_GONE: &str = "process_is_gone";
    pub const WAIT_AUTHORITY_LOST: &str = "wait_authority_lost";
    pub const PUSH_BACK: &str = "push_back";
    pub const NOTIFY_ONE: &str = "notify_one";
    pub const CATCH_UNWIND: &str = "catch_unwind";
    pub const IS_CHILD_ALREADY_REAPED: &str = "is_child_already_reaped";
    pub const SETTLE_CHILD: &str = "settle_child";
    pub const SETTLE_CHILD_ON_DROP: &str = "settle_child_on_drop";
    pub const SETTLEMENT_ACTIONS: &str = "settlement_actions";
    pub const RETAIN_UNSETTLED: &str = "retain_unsettled";
    pub const RETAIN_FOR_DROP: &str = "retain_for_drop";
    pub const RETAIN: &str = "retain";
    pub const ADVANCE: &str = "advance";
    pub const RUN: &str = "run";
    pub const FALLBACK: &str = "fallback";
    pub const RUN_ITERATION: &str = "run_iteration";
    pub const TRY_REAP: &str = "try_reap";
    pub const SWAP_REMOVE: &str = "swap_remove";
    pub const TERMINATION_REQUIRED: &str = "termination_required";
    pub const WORKER_FAILED: &str = "worker_failed";
    pub const TRY_WAIT: &str = "try_wait";
    pub const WAIT_METHOD: &str = "wait";
    pub const KILL_METHOD: &str = "kill";
    pub const START_KILL_METHOD: &str = "start_kill";
    pub const KILL_ON_DROP_METHOD: &str = "kill_on_drop";
    pub const SET_PDEATHSIG_METHOD: &str = "set_pdeathsig";
    pub const PR_SET_PDEATHSIG: &str = "PR_SET_PDEATHSIG";
    pub const PRCTL_SYSCALL: &str = "SYS_prctl";
    pub const LIBC_PRCTL: &str = "prctl";
    pub const SELF_RECEIVER: &str = "self";
    pub const SELF_CHILD_RECEIVER: &str = "self_child";
    pub const SELF_TYPE: &str = "Self";
    pub const CHILD_RECEIVER: &str = "child";
    pub const LIBC_NAMESPACE: &str = "libc";
    pub const LIBC_SYSCALL: &str = "syscall";
    pub const GETTID_SYSCALL: &str = "SYS_gettid";
    pub const PIDFD_OPEN_SYSCALL: &str = "SYS_pidfd_open";
    pub const LIBC_PTRACE: &str = "ptrace";
    pub const PTRACE_SEIZE_FUNCTION: &str = "seize";
    pub const PTRACE_CONTROL_FUNCTION: &str = "ptrace_control";
    pub const PIDFD_SIGNAL_SYSCALL: &str = "SYS_pidfd_send_signal";
    pub const TGKILL_SYSCALL: &str = "SYS_tgkill";
    pub const TKILL_SYSCALL: &str = "SYS_tkill";
    pub const KILL_SYSCALL: &str = "SYS_kill";
    pub const SIGNAL_QUEUE_SYSCALLS: &[&str] = &["SYS_rt_sigqueueinfo", "SYS_rt_tgsigqueueinfo"];
    pub const NO_SIGNAL_CONSTANT: &str = "NO_SIGNAL";
    pub const AUDIT_SIGNAL_SYMBOL: &str = "proc_signal_with_audittoken";
    pub const AUDIT_SIGNAL_FUNCTION: &str = "SignalFunction";
    pub const AUDIT_CALL_BINDING: &str = "function";
    pub const AUDIT_INPUT_BINDING: &str = "capability";
    pub const AUDIT_WRAPPER_FUNCTION: &str = "signal_once_with_capability";
    pub const AUDIT_ENTRY_FUNCTION: &str = "signal_once";
    pub const AUDIT_RESOLVER_FUNCTION: &str = "resolve_signal_function";
    pub const AUDIT_STATIC_BINDING: &str = "SIGNAL_FUNCTION";
    pub const AUDIT_SYMBOL_BINDING: &str = "SIGNAL_SYMBOL";
    pub const AUDIT_CAPABILITY_FUNCTION: &str = "capability_available";
    pub const GET_OR_INIT: &str = "get_or_init";
    pub const IS_SOME_METHOD: &str = "is_some";
    pub const RAW_LIBC_SIGNAL_NAMES: &[&str] = &[
        "kill",
        "killpg",
        "raise",
        "pthread_kill",
        "pthread_sigqueue",
        "sigqueue",
        "pidfd_send_signal",
        LIBC_PTRACE,
    ];
    pub const KILL_EXECUTABLE: &str = "kill";
    pub const PKILL_EXECUTABLE: &str = "pkill";
    pub const KILLALL_EXECUTABLE: &str = "killall";
    pub const SIGNAL_EXECUTABLES: &[&str] =
        &[KILL_EXECUTABLE, PKILL_EXECUTABLE, KILLALL_EXECUTABLE];
    pub const CURRENT_EXEC_THREAD_ID: &str = "current_exec_thread_id";
    pub const TEST_FILE_SUFFIX: &str = "_test";
    pub const TESTS_FILE_SUFFIX: &str = "_tests";
    pub const IMPLICIT_CHILD_TERMINATION_NAMES: &[&str] = &[
        START_KILL_METHOD,
        KILL_ON_DROP_METHOD,
        SET_PDEATHSIG_METHOD,
        PR_SET_PDEATHSIG,
        PRCTL_SYSCALL,
        LIBC_PRCTL,
    ];
}
