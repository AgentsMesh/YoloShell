use super::*;
use std::io::Cursor;

const TEST_EXECUTABLE: &str = "agent helper) worker";
const TEST_FOREGROUND_GROUP: i32 = 17;
const TEST_PARENT_PID: u32 = 11;
const TEST_PID: u32 = 13;
const TEST_PROCESS_GROUP: i32 = TEST_FOREGROUND_GROUP;
const TEST_START_TIME: u64 = 19;
const VALID_STATE: &str = "S";
const MALFORMED_STAT: &[u8] = b"missing executable boundary";

#[test]
fn parses_lineage_groups_executable_and_start_time() {
    let parsed = parse_process_stat(&fixture(TEST_START_TIME)).unwrap();
    assert_eq!(parsed.pid, TEST_PID);
    assert_eq!(parsed.parent_pid, TEST_PARENT_PID);
    assert_eq!(parsed.process_group, TEST_PROCESS_GROUP);
    assert_eq!(parsed.foreground_group, TEST_FOREGROUND_GROUP);
    assert_eq!(parsed.executable, TEST_EXECUTABLE);
    assert_eq!(parsed.start_time, TEST_START_TIME);
}

#[test]
fn rejects_zero_truncated_non_utf8_and_oversized_stats() {
    assert!(parse_process_stat(&fixture(u64::default())).is_none());
    assert!(parse_process_stat(MALFORMED_STAT).is_none());
    let mut non_utf8 = fixture(TEST_START_TIME);
    non_utf8.push(u8::MAX);
    assert!(parse_process_stat(&non_utf8).is_none());
    let oversized = vec![u8::default(); MAX_PROCESS_STAT_BYTES + PROCESS_STAT_READ_SENTINEL_BYTES];
    assert!(parse_process_stat(&oversized).is_none());
}

#[test]
fn bounded_reader_never_returns_bytes_beyond_the_limit() {
    let oversized = vec![u8::default(); MAX_PROCESS_STAT_BYTES + PROCESS_STAT_READ_SENTINEL_BYTES];
    assert!(read_bounded(Cursor::new(oversized)).is_none());
    let valid = fixture(TEST_START_TIME);
    assert_eq!(read_bounded(Cursor::new(&valid)), Some(valid));
}

fn fixture(start_time: u64) -> Vec<u8> {
    let mut fields = vec![u64::default().to_string(); START_TIME_INDEX + 1];
    fields[usize::default()] = VALID_STATE.to_owned();
    fields[PARENT_PID_INDEX] = TEST_PARENT_PID.to_string();
    fields[PROCESS_GROUP_INDEX] = TEST_PROCESS_GROUP.to_string();
    fields[FOREGROUND_GROUP_INDEX] = TEST_FOREGROUND_GROUP.to_string();
    fields[START_TIME_INDEX] = start_time.to_string();
    format!("{TEST_PID} ({TEST_EXECUTABLE}) {}\n", fields.join(" ")).into_bytes()
}
