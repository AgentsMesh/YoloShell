use super::*;

const QUERY_FAILURE_RESULT: i32 = -1;

#[test]
fn unique_metadata_enoent_reports_a_disappeared_candidate() {
    assert!(metadata_with(TEST_PID, absent_query).unwrap().is_none());
}

#[test]
fn unique_metadata_enoent_requires_basic_absence_proof() {
    let error = metadata_with(TEST_PID, absent_then_running_query).unwrap_err();
    assert_eq!(
        error
            .downcast_ref::<std::io::Error>()
            .and_then(std::io::Error::raw_os_error),
        Some(libc::ENOENT)
    );
}

unsafe extern "C" fn absent_query(
    _pid: i32,
    _flavor: i32,
    _argument: u64,
    _buffer: *mut libc::c_void,
    _buffer_size: i32,
) -> i32 {
    fail_query(libc::ENOENT)
}

unsafe extern "C" fn absent_then_running_query(
    pid: i32,
    flavor: i32,
    argument: u64,
    buffer: *mut libc::c_void,
    buffer_size: i32,
) -> i32 {
    if flavor == PROC_PIDT_BSDINFOWITHUNIQID {
        return fail_query(libc::ENOENT);
    }
    if flavor != libc::PROC_PIDTBSDINFO || argument != MACOS_PROC_PIDINFO_NO_ARGUMENT {
        return fail_query(libc::EINVAL);
    }
    let mut raw = unsafe { std::mem::zeroed::<libc::proc_bsdinfo>() };
    raw.pbi_pid = pid as u32;
    raw.pbi_status = libc::SRUN;
    write_response(buffer, buffer_size, raw)
}

fn write_response<T>(buffer: *mut libc::c_void, buffer_size: i32, value: T) -> i32 {
    let Ok(expected) = i32::try_from(std::mem::size_of::<T>()) else {
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

const TEST_PID: u32 = 41;
