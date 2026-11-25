use super::*;

const PROCESS_ID: u32 = 42;
const PARENT_ID: u32 = 41;
const START_TIME: u64 = 4_242;
const REMAINING_FIELDS: &str = "2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18";
const COMM: &str = "fixture (nested) command";

#[test]
fn parent_metadata_uses_same_parser_as_liveness_identity() {
    let stat = format!("{PROCESS_ID} ({COMM}) R {PARENT_ID} {REMAINING_FIELDS} {START_TIME}");
    let parsed = parse_linux_process_stat(&stat).unwrap();
    assert_eq!(
        (parsed.pid, parsed.parent_pid, parsed.start_time),
        (PROCESS_ID, PARENT_ID, START_TIME)
    );
    assert!(parsed.is_active());
}

#[test]
fn malformed_pid_parent_and_empty_command_are_not_ancestry_evidence() {
    let cases = [
        format!("0 ({COMM}) R {PARENT_ID} {REMAINING_FIELDS} {START_TIME}"),
        format!("invalid ({COMM}) R {PARENT_ID} {REMAINING_FIELDS} {START_TIME}"),
        format!("{PROCESS_ID} ({COMM}) R invalid {REMAINING_FIELDS} {START_TIME}"),
        format!("{PROCESS_ID} () R {PARENT_ID} {REMAINING_FIELDS} {START_TIME}"),
    ];
    for stat in cases {
        assert!(parse_linux_process_stat(&stat).is_err());
    }
}
