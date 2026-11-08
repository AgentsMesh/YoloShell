use super::*;
use std::mem::size_of;

const TEST_PID: u32 = 41;
const OTHER_PID: u32 = 42;
const TEST_PARENT_PID: u32 = 40;
const TEST_STATUS: u32 = libc::SRUN;
const TEST_PROCESS_GROUP: u32 = 31;
const TEST_FOREGROUND_GROUP: i32 = -29;
const TEST_START_SECONDS: u64 = 7;
const TEST_START_MICROSECONDS: u64 = 11;
const TEST_UNIQUE_ID: u64 = 101;
const TEST_PARENT_UNIQUE_ID: u64 = 97;
const TEST_PID_VERSION: i32 = -23;
const INVALID_PROCESS_GROUP: u32 = i32::MAX as u32 + 1;
const QUERY_FAILURE_RESULT: i32 = -1;

#[test]
fn successful_unique_metadata_filters_a_zombie() {
    assert!(
        metadata_with(TEST_PID, unique_zombie_query)
            .unwrap()
            .is_none()
    );
}

#[test]
fn permission_denial_requires_explicit_basic_zombie_proof() {
    assert!(
        metadata_with(TEST_PID, permission_then_zombie_query)
            .unwrap()
            .is_none()
    );
    let error = metadata_with(TEST_PID, permission_then_running_query).unwrap_err();
    assert_eq!(
        error
            .downcast_ref::<std::io::Error>()
            .and_then(std::io::Error::raw_os_error),
        Some(libc::EPERM)
    );
}

#[test]
fn zero_response_cannot_reuse_stale_esrch() {
    unsafe { *libc::__error() = libc::ESRCH };
    let error = metadata_with(TEST_PID, zero_without_errno_query).unwrap_err();
    assert!(error.to_string().contains(MISSING_QUERY_ERRNO_DETAIL));
}

#[test]
fn conversion_preserves_atomic_kernel_fields_and_bit_patterns() {
    let metadata = convert(TEST_PID, raw_metadata(TEST_PID, TEST_PROCESS_GROUP)).unwrap();
    assert_eq!(metadata.pid, TEST_PID);
    assert_eq!(metadata.status, TEST_STATUS);
    assert_eq!(metadata.parent_pid, TEST_PARENT_PID);
    assert_eq!(metadata.process_group, TEST_PROCESS_GROUP as i32);
    assert_eq!(metadata.foreground_group, TEST_FOREGROUND_GROUP);
    assert_eq!(
        metadata.start_time,
        TEST_START_SECONDS * MICROSECONDS_PER_SECOND + TEST_START_MICROSECONDS
    );
    assert_eq!(metadata.unique_id, TEST_UNIQUE_ID);
    assert_eq!(metadata.parent_unique_id, TEST_PARENT_UNIQUE_ID);
    assert_eq!(metadata.pid_version, TEST_PID_VERSION as u32);
}

#[test]
fn conversion_rejects_inconsistent_or_unrepresentable_bsd_fields() {
    assert!(convert(TEST_PID, raw_metadata(OTHER_PID, TEST_PROCESS_GROUP)).is_err());
    assert!(convert(TEST_PID, raw_metadata(TEST_PID, INVALID_PROCESS_GROUP)).is_err());
    let mut missing_unique_id = raw_metadata(TEST_PID, TEST_PROCESS_GROUP);
    missing_unique_id.unique.unique_id = u64::default();
    assert!(convert(TEST_PID, missing_unique_id).is_err());
}

fn raw_metadata(pid: u32, process_group: u32) -> ProcBsdInfoWithUniqueId {
    let mut bsd = unsafe { std::mem::zeroed::<libc::proc_bsdinfo>() };
    bsd.pbi_pid = pid;
    bsd.pbi_status = TEST_STATUS;
    bsd.pbi_ppid = TEST_PARENT_PID;
    bsd.pbi_pgid = process_group;
    bsd.e_tpgid = u32::from_ne_bytes(TEST_FOREGROUND_GROUP.to_ne_bytes());
    bsd.pbi_start_tvsec = TEST_START_SECONDS;
    bsd.pbi_start_tvusec = TEST_START_MICROSECONDS;
    ProcBsdInfoWithUniqueId {
        bsd,
        unique: ProcUniqueIdentifierInfo {
            executable_uuid: [u8::default(); EXECUTABLE_UUID_BYTES],
            unique_id: TEST_UNIQUE_ID,
            parent_unique_id: TEST_PARENT_UNIQUE_ID,
            pid_version: TEST_PID_VERSION,
            original_parent_pid_version: i32::default(),
            reserved_2: u64::default(),
            reserved_3: u64::default(),
        },
    }
}

unsafe extern "C" fn unique_zombie_query(
    _pid: i32,
    flavor: i32,
    argument: u64,
    buffer: *mut libc::c_void,
    buffer_size: i32,
) -> i32 {
    if flavor != PROC_PIDT_BSDINFOWITHUNIQID || argument != MACOS_PROC_PIDINFO_NO_ARGUMENT {
        return fail_query(libc::EINVAL);
    }
    let mut raw = raw_metadata(TEST_PID, TEST_PROCESS_GROUP);
    raw.bsd.pbi_status = libc::SZOMB;
    unsafe { write_response(buffer, buffer_size, raw) }
}

unsafe extern "C" fn zero_without_errno_query(
    _pid: i32,
    _flavor: i32,
    _argument: u64,
    _buffer: *mut libc::c_void,
    _buffer_size: i32,
) -> i32 {
    assert_eq!(unsafe { *libc::__error() }, libc::c_int::default());
    MAX_EMPTY_RESPONSE_SIZE
}

unsafe extern "C" fn permission_then_zombie_query(
    pid: i32,
    flavor: i32,
    argument: u64,
    buffer: *mut libc::c_void,
    buffer_size: i32,
) -> i32 {
    unsafe { permission_query(pid, flavor, argument, buffer, buffer_size, libc::SZOMB) }
}

unsafe extern "C" fn permission_then_running_query(
    pid: i32,
    flavor: i32,
    argument: u64,
    buffer: *mut libc::c_void,
    buffer_size: i32,
) -> i32 {
    unsafe { permission_query(pid, flavor, argument, buffer, buffer_size, libc::SRUN) }
}

unsafe fn permission_query(
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
        return fail_query(libc::EPERM);
    }
    if flavor != libc::PROC_PIDTBSDINFO {
        return fail_query(libc::EINVAL);
    }
    let mut raw = raw_metadata(TEST_PID, TEST_PROCESS_GROUP).bsd;
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
