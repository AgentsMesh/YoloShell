use super::{ProcessIdentity, macos_native};
use std::mem::MaybeUninit;
use std::path::Path;
use std::time::{Duration, Instant};

pub(super) const TEST_TIMEOUT: Duration = Duration::from_secs(5);
pub(super) const POLL_INTERVAL: Duration = Duration::from_millis(10);
const WAITID_OPTIONS: i32 = libc::WEXITED | libc::WNOWAIT | libc::WNOHANG;
const NO_EXIT_PID: libc::pid_t = 0;
const WAITID_SUCCESS_RESULT: libc::c_int = 0;
const MARKER_MISSING_DETAIL: &str = "helper marker was not published";
const WAITID_FAILURE_DETAIL: &str = "waitid failed";
const ROOT_EXIT_MISSING_DETAIL: &str = "root did not exit";
const LEAF_REPARENT_MISSING_DETAIL: &str = "leaf was not reparented";

pub(super) fn wait_for_path(path: &Path) {
    let deadline = Instant::now() + TEST_TIMEOUT;
    while !path.exists() {
        assert!(Instant::now() < deadline, "{}", MARKER_MISSING_DETAIL);
        std::thread::sleep(POLL_INTERVAL);
    }
}

pub(super) fn wait_for_exit_unreaped(identity: ProcessIdentity) {
    let deadline = Instant::now() + TEST_TIMEOUT;
    loop {
        let mut information = MaybeUninit::<libc::siginfo_t>::zeroed();
        let result = unsafe {
            libc::waitid(
                libc::P_PID,
                identity.pid as libc::id_t,
                information.as_mut_ptr(),
                WAITID_OPTIONS,
            )
        };
        assert_eq!(result, WAITID_SUCCESS_RESULT, "{}", WAITID_FAILURE_DETAIL);
        if unsafe { information.assume_init().si_pid() } != NO_EXIT_PID {
            return;
        }
        assert!(Instant::now() < deadline, "{}", ROOT_EXIT_MISSING_DETAIL);
        std::thread::sleep(POLL_INTERVAL);
    }
}

pub(super) fn wait_for_reparent(identity: ProcessIdentity, root_pid: u32) {
    let deadline = Instant::now() + TEST_TIMEOUT;
    while macos_native::kernel_process_metadata(identity.pid)
        .unwrap()
        .parent_pid
        == root_pid
    {
        assert!(
            Instant::now() < deadline,
            "{}",
            LEAF_REPARENT_MISSING_DETAIL
        );
        std::thread::sleep(POLL_INTERVAL);
    }
}
