use super::*;

use crate::process::constants::MAX_PROCESS_INVENTORY_IDS;
const INVALID_PROCESS_ID: u32 = 0;
const INVALID_PROCESS_GROUP: i32 = 0;
const EMPTY_PID_LIMIT: usize = 0;
const TEST_PATH: &[u8] = b"/tmp";
const TEST_C_PATH: &[u8] = b"/tmp\0ignored";
const EMPTY_PATH_BYTES: &[u8] = b"";
const SHORT_PATH_BYTES: &[u8] = b"short";
const EXCESS_REPORTED_BYTE_COUNT: usize = 1;
const UNTERMINATED_PATH_BYTES: &[u8] = b"unterminated";
const EMPTY_C_PATH_BYTES: &[u8] = b"\0empty";
const MATCHED_PROCESS_ID: u32 = 1;
const OTHER_PROCESS_ID: u32 = 2;
const MISSING_PROCESS_ID: u32 = i32::MAX as u32;
const SHORT_RESPONSE_DECREMENT: libc::c_int = 1;

#[test]
fn native_metadata_and_paths_observe_the_current_process() {
    let pid = std::process::id();
    let metadata = process_metadata(pid).unwrap();
    assert_eq!(metadata.pid, pid);
    assert_ne!(metadata.parent_pid, INVALID_PROCESS_ID);
    assert!(metadata.process_group.is_positive());
    assert_ne!(metadata.start_time, u64::default());
    let kernel = kernel_process_metadata(pid).unwrap();
    let parent = kernel_process_metadata(kernel.parent_pid).unwrap();
    assert_eq!(kernel.pid, metadata.pid);
    assert_eq!(kernel.start_time, metadata.start_time);
    assert_eq!(
        kernel.unique_id,
        kernel_process_metadata(pid).unwrap().unique_id
    );
    assert_eq!(kernel.parent_unique_id, parent.unique_id);
    assert_ne!(kernel.unique_id, kernel.parent_unique_id);
    assert_eq!(
        kernel.pid_version,
        kernel_process_metadata(pid).unwrap().pid_version
    );
    assert!(executable_path(pid).unwrap().is_absolute());
    assert!(cwd_path(pid).unwrap().is_absolute());
    assert!(process_metadata(INVALID_PROCESS_ID).is_err());
    assert!(process_metadata(i32::MAX as u32).is_err());
    assert!(executable_path(i32::MAX as u32).is_err());
    assert!(executable_path(u32::MAX).is_err());
    assert!(cwd_path(u32::MAX).is_err());
    assert!(kernel_process_metadata(INVALID_PROCESS_ID).is_err());
    assert!(kernel_process_metadata(u32::MAX).is_err());
}

#[test]
fn native_pid_enumeration_is_bounded() {
    let user = unsafe { libc::geteuid() };
    let owned = user_pids(user, MAX_PROCESS_INVENTORY_IDS).unwrap();
    assert!(owned.contains(&std::process::id()));
    let group = unsafe { libc::getpgrp() };
    let members = foreground_group_pids(group, MAX_PROCESS_INVENTORY_IDS).unwrap();
    assert!(!members.is_empty());
    assert!(foreground_group_pids(INVALID_PROCESS_GROUP, MAX_PROCESS_INVENTORY_IDS).is_err());
    assert!(user_pids(user, EMPTY_PID_LIMIT).is_err());
}

#[test]
fn pid_and_path_parsers_reject_corrupt_kernel_results() {
    assert_eq!(
        parse_reported_path(TEST_C_PATH, TEST_C_PATH.len()).unwrap(),
        PathBuf::from(std::str::from_utf8(TEST_PATH).unwrap())
    );
    assert_eq!(
        parse_reported_path(TEST_PATH, TEST_PATH.len()).unwrap(),
        PathBuf::from(std::str::from_utf8(TEST_PATH).unwrap())
    );
    assert_eq!(
        parse_c_path(TEST_C_PATH).unwrap(),
        PathBuf::from(std::str::from_utf8(TEST_PATH).unwrap())
    );
    assert!(parse_reported_path(EMPTY_PATH_BYTES, EMPTY_PATH_BYTES.len()).is_err());
    assert!(
        parse_reported_path(
            SHORT_PATH_BYTES,
            SHORT_PATH_BYTES.len() + EXCESS_REPORTED_BYTE_COUNT,
        )
        .is_err()
    );
    assert!(parse_c_path(UNTERMINATED_PATH_BYTES).is_err());
    assert!(parse_c_path(EMPTY_C_PATH_BYTES).is_err());
}
#[test]
fn current_process_is_not_in_kernel_exit() {
    assert_eq!(
        process_exit_state(std::process::id()).unwrap(),
        ProcessExitState::Running
    );
}

#[test]
fn kernel_exit_query_distinguishes_missing_metadata_and_decodes_the_abi_flag() {
    assert_eq!(
        process_exit_state(MISSING_PROCESS_ID).unwrap(),
        ProcessExitState::MetadataMissing
    );
    assert_eq!(
        process_exit_flag(MATCHED_PROCESS_ID, MATCHED_PROCESS_ID, PROCESS_IN_EXIT_FLAG).unwrap(),
        ProcessExitState::Exiting
    );
    assert_eq!(
        process_exit_flag(MATCHED_PROCESS_ID, MATCHED_PROCESS_ID, u32::default()).unwrap(),
        ProcessExitState::Running
    );
    assert!(ProcessExitState::Exiting.requires_output_progress());
    assert!(ProcessExitState::MetadataMissing.requires_output_progress());
    assert!(!ProcessExitState::Running.requires_output_progress());
    assert!(process_exit_flag(MATCHED_PROCESS_ID, OTHER_PROCESS_ID, PROCESS_IN_EXIT_FLAG).is_err());
    assert!(process_exit_state(INVALID_PROCESS_ID).is_err());
}

#[test]
fn process_info_query_classifies_kernel_failures_and_short_responses() {
    let missing = try_pid_info_with::<libc::proc_bsdinfo>(
        MATCHED_PROCESS_ID,
        libc::PROC_PIDTBSDINFO,
        missing_process_query,
    )
    .unwrap();
    assert!(missing.is_none());

    let denied = try_pid_info_with::<libc::proc_bsdinfo>(
        MATCHED_PROCESS_ID,
        libc::PROC_PIDTBSDINFO,
        denied_process_query,
    )
    .unwrap_err();
    assert!(denied.downcast_ref::<std::io::Error>().is_some());

    let short = try_pid_info_with::<libc::proc_bsdinfo>(
        MATCHED_PROCESS_ID,
        libc::PROC_PIDTBSDINFO,
        short_process_query,
    )
    .unwrap_err();
    assert!(short.to_string().contains(METADATA_RESPONSE_SIZE_DETAIL));

    let missing_errno = try_pid_info_with::<libc::proc_bsdinfo>(
        MATCHED_PROCESS_ID,
        libc::PROC_PIDTBSDINFO,
        missing_errno_query,
    )
    .unwrap_err();
    assert!(
        missing_errno
            .to_string()
            .contains(METADATA_MISSING_ERRNO_DETAIL)
    );
}

unsafe extern "C" fn missing_process_query(
    _pid: libc::c_int,
    _flavor: libc::c_int,
    _argument: u64,
    _buffer: *mut libc::c_void,
    _size: libc::c_int,
) -> libc::c_int {
    unsafe { *libc::__error() = libc::ESRCH };
    EMPTY_METADATA_RESPONSE
}

unsafe extern "C" fn denied_process_query(
    _pid: libc::c_int,
    _flavor: libc::c_int,
    _argument: u64,
    _buffer: *mut libc::c_void,
    _size: libc::c_int,
) -> libc::c_int {
    unsafe { *libc::__error() = libc::EPERM };
    EMPTY_METADATA_RESPONSE
}

unsafe extern "C" fn short_process_query(
    _pid: libc::c_int,
    _flavor: libc::c_int,
    _argument: u64,
    _buffer: *mut libc::c_void,
    size: libc::c_int,
) -> libc::c_int {
    size.saturating_sub(SHORT_RESPONSE_DECREMENT)
}

unsafe extern "C" fn missing_errno_query(
    _pid: libc::c_int,
    _flavor: libc::c_int,
    _argument: u64,
    _buffer: *mut libc::c_void,
    _size: libc::c_int,
) -> libc::c_int {
    EMPTY_METADATA_RESPONSE
}
