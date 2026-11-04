use super::*;

const INVALID_EXIT_CODE: i32 = -1;
const OVERFLOW_SIGNAL: i32 = SIGNAL_EXIT_CODE_BASE as i32;
const OVERSIZED_SIGNAL: i32 = i32::MAX;
const SUCCESS_EXIT_CODE: i32 = 0;

#[test]
fn ordinary_and_signal_exit_codes_are_preserved() {
    assert_eq!(
        exit_code(ExitStatus::from_raw(SUCCESS_EXIT_CODE)),
        ExitCode::SUCCESS
    );
    let expected = ExitCode::from(SIGNAL_EXIT_CODE_BASE + u8::try_from(libc::SIGTERM).unwrap());
    assert_eq!(exit_code(ExitStatus::from_raw(libc::SIGTERM)), expected);
}

#[test]
fn invalid_child_status_fails_closed() {
    assert_eq!(
        normalized_exit_code(Some(INVALID_EXIT_CODE), None),
        ExitCode::FAILURE
    );
    assert_eq!(
        normalized_exit_code(None, Some(OVERSIZED_SIGNAL)),
        ExitCode::FAILURE
    );
    assert_eq!(
        normalized_exit_code(None, Some(OVERFLOW_SIGNAL)),
        ExitCode::FAILURE
    );
    assert_eq!(normalized_exit_code(None, None), ExitCode::FAILURE);
}
