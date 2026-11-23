use super::*;
use portable_pty::{PtySize, native_pty_system};

const TEST_ROWS: u16 = 24;
const TEST_COLUMNS: u16 = 80;
const TEST_PIXEL_SIZE: u16 = 0;

#[test]
fn vanished_pty_path_is_a_conclusive_empty_holder_snapshot() {
    let pair = native_pty_system().openpty(PtySize {
        rows: TEST_ROWS,
        cols: TEST_COLUMNS,
        pixel_width: TEST_PIXEL_SIZE,
        pixel_height: TEST_PIXEL_SIZE,
    });
    let pair = pair.unwrap();
    let path = pair.master.tty_name().unwrap();
    assert!(path.exists());
    drop(pair);
    assert!(!path.exists());
    assert!(
        path_holders(&path, MAX_PTY_PROOF_CANDIDATES)
            .unwrap_err()
            .is_missing_path()
    );
}

#[test]
fn missing_path_errno_is_distinct_but_other_query_failures_remain_errors() {
    set_errno(libc::ENOENT);
    assert!(
        validate_query_result(QUERY_FAILURE_RESULT)
            .unwrap_err()
            .is_missing_path()
    );
    set_errno(libc::EIO);
    assert!(matches!(
        validate_query_result(QUERY_FAILURE_RESULT),
        Err(PathHolderError::Query(error)) if error.raw_os_error() == Some(libc::EIO)
    ));
}

fn set_errno(errno: c_int) {
    unsafe { *__error() = errno };
}
