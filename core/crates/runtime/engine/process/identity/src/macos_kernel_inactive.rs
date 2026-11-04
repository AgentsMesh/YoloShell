use super::{MACOS_PROC_PIDINFO_NO_ARGUMENT, MAX_EMPTY_RESPONSE_SIZE, ProcPidInfo, reset_errno};
use std::mem::{MaybeUninit, size_of};

pub(super) const INACTIVE_CONFIRMATION_ATTEMPTS: usize = 3;

pub(super) fn confirmed_inactive(pid: i32, query: ProcPidInfo) -> bool {
    let Ok(expected) = i32::try_from(size_of::<libc::proc_bsdinfo>()) else {
        return false;
    };
    for _attempt in std::iter::repeat_n((), INACTIVE_CONFIRMATION_ATTEMPTS) {
        let mut raw = MaybeUninit::<libc::proc_bsdinfo>::uninit();
        reset_errno();
        let read = unsafe {
            query(
                pid,
                libc::PROC_PIDTBSDINFO,
                MACOS_PROC_PIDINFO_NO_ARGUMENT,
                raw.as_mut_ptr().cast(),
                expected,
            )
        };
        if read == expected {
            let raw = unsafe { raw.assume_init() };
            return u32::try_from(pid).is_ok_and(|pid| raw.pbi_pid == pid)
                && raw.pbi_status == libc::SZOMB;
        }
        if read <= MAX_EMPTY_RESPONSE_SIZE {
            match std::io::Error::last_os_error().raw_os_error() {
                Some(libc::ESRCH | libc::ENOENT) => return true,
                Some(libc::EPERM | libc::EINVAL) => {}
                _ => return false,
            }
        } else {
            return false;
        }
        std::thread::yield_now();
    }
    false
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../integration_tests/macos_kernel_inactive_tests.rs"]
mod tests;
