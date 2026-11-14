use super::super::super::constants::process_inventory_error;
use super::*;
use std::ffi::OsStr;

const FIRST_PID: u32 = 41;
const FIRST_PID_NAME: &str = "41";
const SECOND_PID: u32 = 43;
const SECOND_PID_NAME: &str = "43";
const ZERO_PID_NAME: &str = "0";
const OVERFLOW_PID_NAME: &str = "4294967296";
const PADDED_FIRST_PID_NAME: &str = "041";
const NON_PROCESS_ENTRY: &str = "sys";
const TEST_PID_LIMIT: usize = 2;

#[test]
fn procfs_inventory_is_bounded_sorted_and_ignores_kernel_entries() {
    let root = tempfile::tempdir().unwrap();
    for name in [SECOND_PID_NAME, NON_PROCESS_ENTRY, FIRST_PID_NAME] {
        std::fs::create_dir(root.path().join(name)).unwrap();
    }
    assert_eq!(
        inventory_at(root.path(), TEST_PID_LIMIT).unwrap(),
        [FIRST_PID, SECOND_PID]
    );
    assert_error(
        inventory_at(root.path(), TEST_PID_LIMIT - 1),
        process_inventory_error::PID_LIMIT,
    );
}

#[test]
fn procfs_inventory_fails_closed_when_root_is_unreadable() {
    let root = tempfile::tempdir().unwrap();
    let missing = root.path().join(NON_PROCESS_ENTRY);
    assert_error(
        inventory_at(&missing, TEST_PID_LIMIT),
        process_inventory_error::OPEN,
    );
}

#[test]
fn procfs_inventory_rejects_aliases_for_the_same_process_id() {
    let root = tempfile::tempdir().unwrap();
    for name in [FIRST_PID_NAME, PADDED_FIRST_PID_NAME] {
        std::fs::create_dir(root.path().join(name)).unwrap();
    }
    assert_error(
        inventory_at(root.path(), TEST_PID_LIMIT),
        process_inventory_error::DUPLICATE_PID,
    );
}

#[test]
fn process_entry_parser_rejects_zero_and_overflow_but_skips_namespaces() {
    assert_eq!(
        parse_pid_name(OsStr::new(FIRST_PID_NAME)).unwrap(),
        Some(FIRST_PID)
    );
    assert_eq!(parse_pid_name(OsStr::new(NON_PROCESS_ENTRY)).unwrap(), None);
    assert_error(
        parse_pid_name(OsStr::new(ZERO_PID_NAME)),
        process_inventory_error::MALFORMED_PID,
    );
    assert_error(
        parse_pid_name(OsStr::new(OVERFLOW_PID_NAME)),
        process_inventory_error::MALFORMED_PID,
    );
}

#[cfg(unix)]
#[test]
fn process_entry_parser_skips_non_utf8_names() {
    use std::os::unix::ffi::OsStrExt;
    assert_eq!(parse_pid_name(OsStr::from_bytes(&[u8::MAX])).unwrap(), None);
}

fn assert_error<T: std::fmt::Debug>(result: Result<T>, expected: &str) {
    assert!(result.unwrap_err().to_string().contains(expected));
}
