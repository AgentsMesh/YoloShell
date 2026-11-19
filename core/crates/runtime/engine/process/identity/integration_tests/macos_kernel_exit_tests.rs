use super::*;
use std::cell::Cell;
use std::mem::size_of;

const TEST_PID: u32 = 51;
const TEST_PROCESS_GROUP: u32 = 37;
const QUERY_FAILURE_RESULT: i32 = -1;
const INITIAL_TRANSITION_STEP: u8 = 0;
const UNIQUE_FAILURE_STEP: u8 = 1;
const AMBIGUOUS_BASIC_STEP: u8 = 2;
const GONE_BASIC_STEP: u8 = 3;
const QUERY_CALL_INCREMENT: u8 = 1;
const EXPECTED_RUNNING_QUERY_CALLS: u8 = 2;
const EXPECTED_PROTECTED_QUERY_CALLS: u8 =
    INACTIVE_CONFIRMATION_ATTEMPTS as u8 + QUERY_CALL_INCREMENT;

thread_local! {
    static TRANSITION_STEP: Cell<u8> = const { Cell::new(INITIAL_TRANSITION_STEP) };
    static QUERY_CALLS: Cell<u8> = const { Cell::new(0) };
}

#[test]
fn einval_requires_explicit_zombie_or_gone_proof() {
    assert!(
        metadata_with(TEST_PID, einval_then_zombie_query)
            .unwrap()
            .is_none()
    );
    assert!(
        metadata_with(TEST_PID, einval_then_gone_query)
            .unwrap()
            .is_none()
    );
    QUERY_CALLS.set(0);
    let error = metadata_with_context(TEST_PID, einval_then_running_query).unwrap_err();
    assert!(error.to_string().contains(&format!("pid={TEST_PID}")));
    assert_eq!(
        error
            .downcast_ref::<std::io::Error>()
            .and_then(std::io::Error::raw_os_error),
        Some(libc::EINVAL)
    );
    assert_eq!(QUERY_CALLS.get(), EXPECTED_RUNNING_QUERY_CALLS);
}

#[test]
fn ambiguous_basic_query_retries_until_esrch_is_explicit() {
    TRANSITION_STEP.set(INITIAL_TRANSITION_STEP);
    assert!(
        metadata_with(TEST_PID, einval_transitioning_to_gone)
            .unwrap()
            .is_none()
    );
    assert_eq!(TRANSITION_STEP.get(), GONE_BASIC_STEP);
}

#[test]
fn persistent_protected_process_exhausts_confirmation_and_fails_closed() {
    QUERY_CALLS.set(0);
    let error = metadata_with(TEST_PID, persistent_permission_denial).unwrap_err();
    assert_eq!(
        error
            .downcast_ref::<std::io::Error>()
            .and_then(std::io::Error::raw_os_error),
        Some(libc::EPERM)
    );
    assert_eq!(QUERY_CALLS.get(), EXPECTED_PROTECTED_QUERY_CALLS);
}

unsafe extern "C" fn einval_transitioning_to_gone(
    _pid: i32,
    flavor: i32,
    _argument: u64,
    _buffer: *mut libc::c_void,
    _buffer_size: i32,
) -> i32 {
    let step = TRANSITION_STEP.get();
    if flavor == PROC_PIDT_BSDINFOWITHUNIQID && step == INITIAL_TRANSITION_STEP {
        TRANSITION_STEP.set(UNIQUE_FAILURE_STEP);
        return fail_query(libc::EINVAL);
    }
    if flavor == libc::PROC_PIDTBSDINFO && step == UNIQUE_FAILURE_STEP {
        TRANSITION_STEP.set(AMBIGUOUS_BASIC_STEP);
        return fail_query(libc::EINVAL);
    }
    TRANSITION_STEP.set(GONE_BASIC_STEP);
    fail_query(libc::ESRCH)
}

unsafe extern "C" fn einval_then_zombie_query(
    pid: i32,
    flavor: i32,
    argument: u64,
    buffer: *mut libc::c_void,
    buffer_size: i32,
) -> i32 {
    unsafe { failure_then_status_query(pid, flavor, argument, buffer, buffer_size, libc::SZOMB) }
}

unsafe extern "C" fn einval_then_running_query(
    pid: i32,
    flavor: i32,
    argument: u64,
    buffer: *mut libc::c_void,
    buffer_size: i32,
) -> i32 {
    QUERY_CALLS.set(QUERY_CALLS.get() + QUERY_CALL_INCREMENT);
    unsafe { failure_then_status_query(pid, flavor, argument, buffer, buffer_size, libc::SRUN) }
}

unsafe extern "C" fn persistent_permission_denial(
    _pid: i32,
    _flavor: i32,
    _argument: u64,
    _buffer: *mut libc::c_void,
    _buffer_size: i32,
) -> i32 {
    QUERY_CALLS.set(QUERY_CALLS.get() + QUERY_CALL_INCREMENT);
    fail_query(libc::EPERM)
}

unsafe extern "C" fn einval_then_gone_query(
    _pid: i32,
    flavor: i32,
    _argument: u64,
    _buffer: *mut libc::c_void,
    _buffer_size: i32,
) -> i32 {
    fail_query(if flavor == PROC_PIDT_BSDINFOWITHUNIQID {
        libc::EINVAL
    } else {
        libc::ESRCH
    })
}

unsafe fn failure_then_status_query(
    _pid: i32,
    flavor: i32,
    argument: u64,
    buffer: *mut libc::c_void,
    buffer_size: i32,
    fallback_status: u32,
) -> i32 {
    if argument != MACOS_PROC_PIDINFO_NO_ARGUMENT {
        return fail_query(libc::EINVAL);
    }
    if flavor == PROC_PIDT_BSDINFOWITHUNIQID {
        return fail_query(libc::EINVAL);
    }
    if flavor != libc::PROC_PIDTBSDINFO {
        return fail_query(libc::EINVAL);
    }
    let mut raw = unsafe { std::mem::zeroed::<libc::proc_bsdinfo>() };
    raw.pbi_pid = TEST_PID;
    raw.pbi_pgid = TEST_PROCESS_GROUP;
    raw.pbi_status = fallback_status;
    unsafe { write_response(buffer, buffer_size, raw) }
}

unsafe fn write_response<T>(buffer: *mut libc::c_void, buffer_size: i32, value: T) -> i32 {
    let Ok(expected) = i32::try_from(size_of::<T>()) else {
        return fail_query(libc::EOVERFLOW);
    };
    if buffer.is_null() || buffer_size != expected {
        return fail_query(libc::EINVAL);
    }
    unsafe { buffer.cast::<T>().write(value) };
    expected
}

fn fail_query(error: i32) -> i32 {
    unsafe { *libc::__error() = error };
    QUERY_FAILURE_RESULT
}
