use super::*;
use std::io::Cursor;

#[test]
fn linux_argv_reader_enforces_raw_and_termination_budgets() {
    use super::super::constants::{
        MAX_SINGLE_PROCESS_ARGV_BYTES, PROCESS_ARGV_READ_SENTINEL_BYTES, process_argv_error,
    };
    let terminated = format!(
        "{}\0{}\0",
        crate::provider_fixture::EXECUTABLE,
        crate::provider_fixture::RESUME_ARGUMENT,
    );
    assert_eq!(
        read_linux_argv(Cursor::new(terminated)).unwrap(),
        [
            crate::provider_fixture::EXECUTABLE,
            crate::provider_fixture::RESUME_ARGUMENT,
        ]
    );
    let oversized = vec![
        super::super::constants::NUL_BYTE;
        MAX_SINGLE_PROCESS_ARGV_BYTES + PROCESS_ARGV_READ_SENTINEL_BYTES
    ];
    assert_eq!(
        read_linux_argv(Cursor::new(oversized))
            .unwrap_err()
            .to_string(),
        process_argv_error::RAW_LIMIT
    );
    assert_eq!(
        read_linux_argv(Cursor::new(crate::provider_fixture::EXECUTABLE.as_bytes(),))
            .unwrap_err()
            .to_string(),
        process_argv_error::UNTERMINATED
    );
}
