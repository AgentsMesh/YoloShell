use super::*;
use std::fs;

const PID: u32 = 17;
const PARENT_PID: u32 = 11;
const PROCESS_GROUP: u32 = 13;
const SESSION_ID: u32 = 7;
const START_TIME: u64 = 19;
const SECOND_PID: u32 = 29;
const SECOND_START_TIME: u64 = 31;
const FIRST_ENTRY_INDEX: usize = 0;
const SECOND_ENTRY_INDEX: usize = 1;
const TEST_ENTRY_LIMIT: usize = 8;
const TEST_PROCESS_LIMIT: usize = 2;
const EXCESS_PROCESS_LIMIT: usize = 0;
const EXCESS_ENTRY_LIMIT: usize = 1;
const READ_LIMIT: usize = 3;
const NON_NUMERIC_ENTRY: &str = "not-a-pid";

#[test]
fn proc_inventory_parses_filters_and_sorts_process_metadata() {
    let root = tempfile::tempdir().unwrap();
    let owner = unsafe { libc::geteuid() };
    write_process(root.path(), SECOND_PID, SECOND_START_TIME, owner);
    write_process(root.path(), PID, START_TIME, owner);
    fs::create_dir(root.path().join(NON_NUMERIC_ENTRY)).unwrap();

    let entries = inventory_at(root.path(), owner, TEST_ENTRY_LIMIT, TEST_PROCESS_LIMIT).unwrap();
    assert_eq!(entries.len(), TEST_PROCESS_LIMIT);
    assert_eq!(entries[FIRST_ENTRY_INDEX].pid, PID);
    assert_eq!(entries[FIRST_ENTRY_INDEX].parent_pid, PARENT_PID);
    assert_eq!(entries[FIRST_ENTRY_INDEX].process_group, PROCESS_GROUP);
    assert_eq!(entries[FIRST_ENTRY_INDEX].session_id, SESSION_ID);
    assert_eq!(entries[FIRST_ENTRY_INDEX].start_time, START_TIME);
    assert_eq!(entries[FIRST_ENTRY_INDEX].effective_user_id, owner);
    assert_eq!(entries[SECOND_ENTRY_INDEX].pid, SECOND_PID);
}

#[test]
fn proc_inventory_fails_closed_when_process_budget_is_exhausted() {
    let root = tempfile::tempdir().unwrap();
    let owner = unsafe { libc::geteuid() };
    write_process(root.path(), PID, START_TIME, owner);
    assert!(inventory_at(root.path(), owner, TEST_ENTRY_LIMIT, EXCESS_PROCESS_LIMIT).is_err());
}

#[test]
fn parsers_reject_mismatched_or_malformed_kernel_records() {
    assert!(parse_stat(&stat(PID, START_TIME), PID).is_ok());
    assert!(parse_stat(&stat(PID, START_TIME), PID.wrapping_add(1)).is_err());
    assert!(parse_stat("malformed", PID).is_err());
    for name in ["", "17x", "-17", " 17"] {
        assert_eq!(parse_pid(std::ffi::OsStr::new(name)), None);
    }
}

#[test]
fn foreign_missing_and_unstable_processes_are_excluded() {
    let root = tempfile::tempdir().unwrap();
    let owner = unsafe { libc::geteuid() };
    write_process(root.path(), PID, START_TIME, owner);
    assert!(
        read_entry(root.path(), SECOND_PID, owner)
            .unwrap()
            .is_none()
    );
    assert!(
        read_entry(root.path(), PID, owner.wrapping_add(1))
            .unwrap()
            .is_none()
    );
    let before = parse_stat(&stat(PID, START_TIME), PID).unwrap();
    let after = parse_stat(&stat(PID, SECOND_START_TIME), PID).unwrap();
    assert!(entry_from_observations(before, owner, after, owner).is_none());
    assert!(entry_from_observations(before, owner.wrapping_add(1), before, owner).is_none());
}

#[test]
fn entry_and_read_limits_fail_closed() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join(NON_NUMERIC_ENTRY)).unwrap();
    fs::create_dir(root.path().join("another-nonnumeric-entry")).unwrap();
    let owner = unsafe { libc::geteuid() };
    assert!(inventory_at(root.path(), owner, EXCESS_ENTRY_LIMIT, TEST_PROCESS_LIMIT).is_err());
    let oversized = root.path().join("oversized");
    fs::write(&oversized, b"four").unwrap();
    assert!(read_bounded(&oversized, READ_LIMIT).is_err());
}

#[test]
fn only_disappearance_errors_are_treated_as_process_churn() {
    let missing = anyhow::Error::from(std::io::Error::from(std::io::ErrorKind::NotFound));
    let exited = anyhow::Error::from(std::io::Error::from_raw_os_error(libc::ESRCH));
    let denied = anyhow::Error::from(std::io::Error::from(std::io::ErrorKind::PermissionDenied));
    assert!(disappeared(&missing));
    assert!(disappeared(&exited));
    assert!(!disappeared(&denied));
}

fn write_process(root: &Path, pid: u32, start_time: u64, owner: u32) {
    let process = root.join(pid.to_string());
    fs::create_dir(&process).unwrap();
    fs::write(process.join(STAT_ENTRY), stat(pid, start_time)).unwrap();
    fs::write(
        process.join(crate::process::constants::LINUX_PROC_STATUS_ENTRY),
        format!("Name:\tfixture\nUid:\t3\t{owner}\t5\t7\n"),
    )
    .unwrap();
}

fn stat(pid: u32, start_time: u64) -> String {
    format!(
        "{pid} (fixture command) R {PARENT_PID} {PROCESS_GROUP} {SESSION_ID} 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 {start_time} 0\n"
    )
}
