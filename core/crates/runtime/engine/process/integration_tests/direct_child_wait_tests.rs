use super::DirectChildExitStatus;

const TEST_EXIT_CODE: libc::c_int = 23;
const WAIT_STATUS_EXIT_CODE_SHIFT: u32 = 8;

#[test]
fn raw_exit_status_preserves_semantic_exit_code() {
    let raw_status = TEST_EXIT_CODE << WAIT_STATUS_EXIT_CODE_SHIFT;
    let status = DirectChildExitStatus::from_wait_status(raw_status);

    assert_eq!(status.code(), Some(TEST_EXIT_CODE));
    assert_eq!(status.signal(), None);
}

#[test]
fn raw_signal_status_does_not_fabricate_exit_code() {
    let status = DirectChildExitStatus::from_wait_status(libc::SIGKILL);

    assert_eq!(status.code(), None);
    assert_eq!(status.signal(), Some(libc::SIGKILL));
}
