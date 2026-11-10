use super::process_supervision_tests::Fixture;
use crate::constants::MAX_PROCESS_INPUT_BYTES;
use std::fs;

const EXECUTABLE: &str = "/usr/bin/touch";
const MARKER: &str = "command-dispatched";
const DESCRIPTION: &str = "process admission boundary";
const EMPTY_OUTPUT_LIMIT: usize = 0;
const OVERSIZE_INPUT_BYTES: usize = MAX_PROCESS_INPUT_BYTES + 1;
const INPUT_BYTE: u8 = b'x';

#[test]
fn invalid_io_budgets_reject_before_a_real_executable_can_mutate_anything() {
    let fixture = Fixture::new();
    let evidence = tempfile::tempdir().unwrap();
    let marker = evidence.path().join(MARKER);
    let command = fixture.command(EXECUTABLE, vec![marker.display().to_string()], DESCRIPTION);
    let input_error = command
        .clone()
        .with_input(vec![INPUT_BYTE; OVERSIZE_INPUT_BYTES])
        .execute()
        .unwrap_err();
    assert!(
        input_error
            .to_string()
            .contains("input exceeded the bounded limit")
    );
    assert!(!marker.exists());
    let output_error = command
        .clone()
        .with_output_limit(EMPTY_OUTPUT_LIMIT)
        .execute()
        .unwrap_err();
    assert!(
        output_error
            .to_string()
            .contains("output limit must be positive")
    );
    assert!(!marker.exists());
    assert!(command.execute().unwrap().status.success());
    assert!(fs::metadata(marker).unwrap().is_file());
}
