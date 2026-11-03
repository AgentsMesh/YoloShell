use super::*;
use std::mem::size_of;

const TEST_PID: i32 = 51;
const OTHER_PID: u32 = 52;
const QUERY_FAILURE_RESULT: i32 = -1;

#[test]
fn mismatched_zombie_response_does_not_prove_candidate_exit() {
    assert!(!confirmed_inactive(TEST_PID, mismatched_zombie_query));
}

unsafe extern "C" fn mismatched_zombie_query(
    _pid: i32,
    flavor: i32,
    argument: u64,
    buffer: *mut libc::c_void,
    buffer_size: i32,
) -> i32 {
    if flavor != libc::PROC_PIDTBSDINFO || argument != MACOS_PROC_PIDINFO_NO_ARGUMENT {
        return fail_query(libc::EINVAL);
    }
    let mut raw = unsafe { std::mem::zeroed::<libc::proc_bsdinfo>() };
    raw.pbi_pid = OTHER_PID;
    raw.pbi_status = libc::SZOMB;
    let expected = i32::try_from(size_of::<libc::proc_bsdinfo>()).unwrap();
    if buffer.is_null() || buffer_size != expected {
        return fail_query(libc::EINVAL);
    }
    unsafe { buffer.cast::<libc::proc_bsdinfo>().write(raw) };
    expected
}

fn fail_query(error: i32) -> i32 {
    unsafe { *libc::__error() = error };
    QUERY_FAILURE_RESULT
}
