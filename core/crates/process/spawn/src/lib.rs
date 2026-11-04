use anyhow::{Context, Result, ensure};
#[cfg(any(test, feature = "yolo-test-support"))]
use std::cell::Cell;
use std::mem::MaybeUninit;
use std::ptr;

const SIGACTION_SUCCESS: libc::c_int = 0;
const SIGCHLD_READ_CONTEXT: &str = "read SIGCHLD disposition";
const SIGCHLD_SET_CONTEXT: &str = "set waitable SIGCHLD disposition";
const SIGCHLD_VERIFY_READ_CONTEXT: &str = "read back waitable SIGCHLD disposition";
const SIGCHLD_WAITABLE_VERIFY_ERROR: &str =
    "SIGCHLD disposition is not waitable after normalization";
const SIGCHLD_FLAGS_VERIFY_ERROR: &str =
    "SIGCHLD disposition retains child auto-reap after normalization";
const HANDLER_FLAGS_TO_CLEAR: libc::c_int = libc::SA_NOCLDWAIT | libc::SA_SIGINFO;
pub const PRIVATE_INPUT_DEADLINE_CONTEXT: &str = "detached process private input deadline expired";

#[cfg(any(test, feature = "yolo-test-support"))]
thread_local! {
    static NORMALIZATION_OBSERVED: Cell<bool> = const { Cell::new(false) };
}

#[path = "spawn_gate.rs"]
mod spawn_gate;
pub use spawn_gate::{
    DescriptorScanGuard, WaitableChildSpawnBoundary, try_descriptor_scan_guard,
    try_waitable_child_spawn_guard, waitable_child_spawn_guard_before,
    WaitableChildSpawnGuard, waitable_child_spawn_guard,
};
#[cfg(any(test, feature = "yolo-test-support"))]
pub use spawn_gate::{ReservedTestSpawnWaiter, reserve_spawn_waiter_for_test};

pub fn normalize_sigchld_waitability() -> Result<()> {
    #[cfg(any(test, feature = "yolo-test-support"))]
    test_hook::inject_normalization_failure()?;
    #[cfg(any(test, feature = "yolo-test-support"))]
    NORMALIZATION_OBSERVED.with(|observed| observed.set(true));
    let mut action = read_sigchld_action().context(SIGCHLD_READ_CONTEXT)?;
    if action.sa_sigaction == libc::SIG_IGN {
        action.sa_sigaction = libc::SIG_DFL;
        action.sa_flags &= !HANDLER_FLAGS_TO_CLEAR;
    } else {
        // A real SIGCHLD handler belongs to the async runtime; preserve it and
        // only remove the flag that would auto-reap children. Replacing it
        // with SIG_DFL breaks tokio's child waiter on macOS after one runtime.
        action.sa_flags &= !libc::SA_NOCLDWAIT;
    }
    set_sigchld_action(&action).context(SIGCHLD_SET_CONTEXT)?;

    let verified = read_sigchld_action().context(SIGCHLD_VERIFY_READ_CONTEXT)?;
    ensure!(
        verified.sa_sigaction != libc::SIG_IGN,
        SIGCHLD_WAITABLE_VERIFY_ERROR
    );
    ensure!(
        verified.sa_flags & libc::SA_NOCLDWAIT == libc::c_int::default(),
        SIGCHLD_FLAGS_VERIFY_ERROR
    );
    Ok(())
}

pub fn read_sigchld_action() -> std::io::Result<libc::sigaction> {
    let mut action = MaybeUninit::<libc::sigaction>::uninit();
    let result = unsafe { libc::sigaction(libc::SIGCHLD, ptr::null(), action.as_mut_ptr()) };
    if result != SIGACTION_SUCCESS {
        return Err(std::io::Error::last_os_error());
    }
    Ok(unsafe { action.assume_init() })
}

pub fn set_sigchld_action(action: &libc::sigaction) -> std::io::Result<()> {
    let result = unsafe { libc::sigaction(libc::SIGCHLD, action, ptr::null_mut()) };
    if result == SIGACTION_SUCCESS {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(test)]
#[cfg(feature = "yolo-integration-tests")]
#[path = "../integration_tests/spawn_lock_tests.rs"]
mod spawn_lock_tests;

#[cfg(any(test, feature = "yolo-test-support"))]
#[path = "../test_support/hook.rs"]
pub mod test_hook;

#[cfg(any(test, feature = "yolo-test-support"))]
pub fn reject_spawn_gate_entry_for_test() -> test_hook::RejectSpawnGateEntry {
    test_hook::reject_spawn_gate_entry()
}

#[cfg(any(test, feature = "yolo-test-support"))]
pub fn clear_normalization_observation() {
    NORMALIZATION_OBSERVED.with(|observed| observed.set(false));
}

#[cfg(any(test, feature = "yolo-test-support"))]
pub fn normalization_was_observed() -> bool {
    NORMALIZATION_OBSERVED.with(Cell::get)
}

fn require_budget(deadline: std::time::Instant) -> Result<()> {
    ensure!(
        std::time::Instant::now() < deadline,
        PRIVATE_INPUT_DEADLINE_CONTEXT
    );
    Ok(())
}
