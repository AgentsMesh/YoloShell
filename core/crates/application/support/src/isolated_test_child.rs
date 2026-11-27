use std::os::unix::process::ExitStatusExt;
use std::process::{ExitCode, ExitStatus};

const SIGNAL_EXIT_CODE_BASE: u8 = 128;

pub fn exit_code(status: ExitStatus) -> ExitCode {
    normalized_exit_code(status.code(), status.signal())
}

fn normalized_exit_code(code: Option<i32>, signal: Option<i32>) -> ExitCode {
    code.and_then(|value| u8::try_from(value).ok())
        .or_else(|| {
            signal
                .and_then(|value| u8::try_from(value).ok())
                .and_then(|value| SIGNAL_EXIT_CODE_BASE.checked_add(value))
        })
        .map(ExitCode::from)
        .unwrap_or(ExitCode::FAILURE)
}

#[cfg(test)]
#[cfg(feature = "yolo-unit-tests")]
#[path = "../unit_tests/isolated_test_child_tests.rs"]
mod tests;
