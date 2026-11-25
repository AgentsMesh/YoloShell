use super::*;
use std::mem::size_of;

const TEST_LIMIT: usize = 3;
const TEST_KIND: u32 = MACOS_PROC_UID_ONLY;
const TEST_SELECTOR: u32 = 71;
const FIRST_TEST_PID: libc::pid_t = 10;
const SECOND_TEST_PID: libc::pid_t = 12;
const TRAILING_TEST_PID: libc::pid_t = 13;
const INVALID_PID_VALUE: libc::pid_t = -1;
const INVALID_BYTE_COUNT: i32 = -1;
const MISALIGNED_BYTE_COUNT: i32 = 1;
const SINGLE_PID_COUNT: usize = 1;
const SHORT_PID_BUFFER_LENGTH: usize = 2;
const INJECTED_QUERY_ERROR: i32 = libc::EIO;

#[test]
fn zero_result_distinguishes_an_empty_inventory_from_failure() {
    unsafe { *libc::__error() = libc::EPERM };
    assert!(
        list_pids_with(TEST_KIND, TEST_SELECTOR, TEST_LIMIT, empty_query)
            .unwrap()
            .is_empty()
    );

    let error = list_pids_with(TEST_KIND, TEST_SELECTOR, TEST_LIMIT, failed_query).unwrap_err();
    assert_eq!(raw_os_error(&error), Some(INJECTED_QUERY_ERROR));
}

#[test]
fn byte_counted_pid_results_are_bounded_and_aligned() {
    let values = [
        FIRST_TEST_PID,
        libc::pid_t::default(),
        SECOND_TEST_PID,
        TRAILING_TEST_PID,
    ];
    let bytes = (TEST_LIMIT * size_of::<libc::pid_t>()) as i32;
    assert_eq!(
        parse_byte_counted_pids(&values, bytes, TEST_LIMIT).unwrap(),
        [FIRST_TEST_PID as u32, SECOND_TEST_PID as u32]
    );
    assert!(parse_byte_counted_pids(&values, INVALID_BYTE_COUNT, TEST_LIMIT).is_err());
    assert!(parse_byte_counted_pids(&values, MISALIGNED_BYTE_COUNT, TEST_LIMIT).is_err());
    assert!(
        parse_byte_counted_pids(&values[..SHORT_PID_BUFFER_LENGTH], bytes, TEST_LIMIT).is_err()
    );
    assert!(parse_pid_values(&[INVALID_PID_VALUE], SINGLE_PID_COUNT, TEST_LIMIT).is_err());
}

unsafe extern "C" fn empty_query(
    _kind: u32,
    _selector: u32,
    _buffer: *mut libc::c_void,
    _bytes: i32,
) -> i32 {
    EMPTY_QUERY_RESULT
}

unsafe extern "C" fn failed_query(
    _kind: u32,
    _selector: u32,
    _buffer: *mut libc::c_void,
    _bytes: i32,
) -> i32 {
    unsafe { *libc::__error() = INJECTED_QUERY_ERROR };
    EMPTY_QUERY_RESULT
}

fn raw_os_error(error: &anyhow::Error) -> Option<i32> {
    error.chain().find_map(|cause| {
        cause
            .downcast_ref::<std::io::Error>()
            .and_then(std::io::Error::raw_os_error)
    })
}
