use super::*;
use std::ffi::CStr;

const TEST_PATH: &str = "/dev/ttys-test";
const EMPTY_TEST_PATH: &str = "";
const NUL_TEST_PATH: &str = "invalid\0path";
const EXPECTED_QUERY_ERROR_DETAIL: &str = "expected query error";
const LIBPROC_QUERY_PANIC_DETAIL: &str = "libproc must not be queried";
const TEST_QUERY_ERRNO: c_int = 5;
const TEST_LIMIT: usize = 3;
const RETURNED_PID_COUNT: usize = 3;
const SINGLE_PID_COUNT: usize = 1;
const FIRST_PID: RawPid = 17;
const SECOND_PID: RawPid = 19;
const INVALID_NEGATIVE_PID: RawPid = -1;
const MISALIGNED_BYTES: c_int = 1;
const UNEXPECTED_NEGATIVE_RESULT: c_int = -2;

#[test]
fn query_contract_is_all_process_flagged_sentinel_bounded_and_deduplicated() {
    let result = path_holders_with(
        Path::new(TEST_PATH),
        TEST_LIMIT,
        |kind, selector, path, flags, buffer, bytes| {
            assert_eq!(kind, MACOS_PROC_ALL_PIDS);
            assert_eq!(selector, PROC_ALL_PIDS_TYPEINFO);
            assert_eq!(flags, PROC_LISTPIDSPATH_EXCLUDE_EVTONLY);
            assert_eq!(
                unsafe { CStr::from_ptr(path) }.to_bytes(),
                TEST_PATH.as_bytes()
            );
            assert_eq!(
                bytes as usize,
                (TEST_LIMIT + PID_LIST_SENTINEL_COUNT) * size_of::<RawPid>()
            );
            write_pids(buffer, &[FIRST_PID, FIRST_PID, SECOND_PID]);
            byte_count(RETURNED_PID_COUNT)
        },
    )
    .unwrap();

    assert_eq!(result, HashSet::from([FIRST_PID as u32, SECOND_PID as u32]));
}

#[test]
fn zero_result_resets_errno_but_ignores_internal_stale_errno() {
    set_errno(c_int::MAX);
    let empty = injected_query(|_, _, _, _, _, _| {
        assert_eq!(current_errno(), SUCCESS_ERRNO);
        EMPTY_QUERY_RESULT
    })
    .unwrap();
    assert!(empty.is_empty());

    let empty = injected_query(|_, _, _, _, _, _| {
        set_errno(TEST_QUERY_ERRNO);
        EMPTY_QUERY_RESULT
    })
    .unwrap();
    assert!(empty.is_empty());
}

#[test]
fn query_failure_preserves_errno_and_missing_errno_is_an_invariant() {
    let error = injected_query(|_, _, _, _, _, _| {
        set_errno(TEST_QUERY_ERRNO);
        QUERY_FAILURE_RESULT
    })
    .unwrap_err();
    let PathHolderError::Query(error) = error else {
        panic!("{EXPECTED_QUERY_ERROR_DETAIL}");
    };
    assert_eq!(error.raw_os_error(), Some(TEST_QUERY_ERRNO));

    let error = injected_query(|_, _, _, _, _, _| QUERY_FAILURE_RESULT).unwrap_err();
    assert!(matches!(
        error,
        PathHolderError::Invariant(MISSING_ERRNO_DETAIL)
    ));
}

#[test]
fn caller_budget_and_sentinel_growth_fail_before_truncation() {
    let error = path_holders_with(
        Path::new(TEST_PATH),
        MAX_PTY_PROOF_CANDIDATES + PID_LIST_SENTINEL_COUNT,
        |_, _, _, _, _, _| panic!("{LIBPROC_QUERY_PANIC_DETAIL}"),
    )
    .unwrap_err();
    assert!(matches!(
        error,
        PathHolderError::BudgetExceeded(LIMIT_DETAIL)
    ));

    let error = injected_query(|_, _, _, _, buffer, _| {
        write_pids(buffer, &[FIRST_PID, SECOND_PID, FIRST_PID, SECOND_PID]);
        byte_count(TEST_LIMIT + PID_LIST_SENTINEL_COUNT)
    })
    .unwrap_err();
    assert!(matches!(
        error,
        PathHolderError::BudgetExceeded(LIMIT_DETAIL)
    ));
}

#[test]
fn malformed_shapes_and_nonpositive_pids_fail_as_invariants() {
    let misaligned = injected_query(|_, _, _, _, _, _| MISALIGNED_BYTES).unwrap_err();
    assert!(matches!(
        misaligned,
        PathHolderError::Invariant(MISALIGNED_RESULT_DETAIL)
    ));

    let overreported = injected_query(|_, _, _, _, _, bytes| bytes + byte_count(1)).unwrap_err();
    assert!(matches!(
        overreported,
        PathHolderError::Invariant(BUFFER_EXCEEDED_DETAIL)
    ));

    let negative = injected_query(|_, _, _, _, _, _| UNEXPECTED_NEGATIVE_RESULT).unwrap_err();
    assert!(matches!(
        negative,
        PathHolderError::Invariant(NEGATIVE_RESULT_DETAIL)
    ));

    for raw in [RawPid::default(), INVALID_NEGATIVE_PID] {
        let error = injected_query(|_, _, _, _, buffer, _| {
            write_pids(buffer, &[raw]);
            byte_count(SINGLE_PID_COUNT)
        })
        .unwrap_err();
        assert!(matches!(
            error,
            PathHolderError::Invariant(NONPOSITIVE_PID_DETAIL)
        ));
    }
}

#[test]
fn path_and_buffer_arithmetic_invariants_fail_closed() {
    for path in [Path::new(EMPTY_TEST_PATH), Path::new(NUL_TEST_PATH)] {
        let error = path_holders_with(path, TEST_LIMIT, |_, _, _, _, _, _| {
            panic!("{LIBPROC_QUERY_PANIC_DETAIL}")
        })
        .unwrap_err();
        assert!(matches!(error, PathHolderError::Invariant(PATH_DETAIL)));
    }
    assert!(matches!(
        pid_buffer(usize::MAX).unwrap_err(),
        PathHolderError::BudgetExceeded(BUFFER_OVERFLOW_DETAIL)
    ));
    let range_limit = (c_int::MAX as usize / size_of::<RawPid>()) + PID_LIST_SENTINEL_COUNT;
    assert!(matches!(
        pid_buffer(range_limit).unwrap_err(),
        PathHolderError::BudgetExceeded(BUFFER_RANGE_DETAIL)
    ));
}

#[test]
fn path_holder_error_display_and_sources_preserve_the_error_class() {
    let query = PathHolderError::Query(std::io::Error::from_raw_os_error(TEST_QUERY_ERRNO));
    assert!(query.to_string().contains(QUERY_ERROR_KIND));
    assert!(std::error::Error::source(&query).is_some());

    for (error, kind) in [
        (
            PathHolderError::BudgetExceeded(LIMIT_DETAIL),
            BUDGET_ERROR_KIND,
        ),
        (
            PathHolderError::Invariant(PATH_DETAIL),
            INVARIANT_ERROR_KIND,
        ),
    ] {
        assert!(error.to_string().contains(kind));
        assert!(std::error::Error::source(&error).is_none());
    }
}

fn injected_query<Q>(query: Q) -> PathHolderResult<HashSet<u32>>
where
    Q: FnMut(c_uint, c_uint, *const c_char, c_uint, *mut c_void, c_int) -> c_int,
{
    path_holders_with(Path::new(TEST_PATH), TEST_LIMIT, query)
}

fn byte_count(pids: usize) -> c_int {
    (pids * size_of::<RawPid>()) as c_int
}

fn write_pids(buffer: *mut c_void, pids: &[RawPid]) {
    unsafe { std::ptr::copy_nonoverlapping(pids.as_ptr(), buffer.cast::<RawPid>(), pids.len()) };
}

fn set_errno(errno: c_int) {
    unsafe { *__error() = errno };
}
