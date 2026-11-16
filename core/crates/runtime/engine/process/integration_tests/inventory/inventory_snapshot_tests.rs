use super::*;
use std::collections::VecDeque;

const TEST_PID: u32 = 41;
const TEST_PARENT_PID: u32 = 37;
const TEST_PROCESS_GROUP: i32 = 31;
const TEST_SESSION_ID: u32 = 29;
const TEST_START_TIME: u64 = 23;
const FIRST_UNIQUE_ID: u64 = 19;
const REUSED_UNIQUE_ID: u64 = 17;
const TEST_PARENT_UNIQUE_ID: u64 = 13;
const TEST_PID_VERSION: u32 = 11;
const STABLE_CANDIDATE_DETAIL: &str = "stable inventory candidate";
const VALID_CANDIDATE_DETAIL: &str = "valid stable inventory candidate";
const METADATA_SNAPSHOT_DETAIL: &str = "metadata snapshot";
const CANDIDATE_RESULT_DETAIL: &str = "inventory candidate result";
const INJECTED_INITIAL_METADATA_FAILURE: &str = "injected initial metadata failure";
const INJECTED_METADATA_FAILURE: &str = "injected post-session metadata failure";
const INJECTED_SESSION_FAILURE: &str = "injected session lookup failure";
const INVALID_PROCESS_GROUP: i32 = -1;

#[test]
fn session_lookup_cannot_join_two_pid_generations() {
    let result = injected_entry([
        Some(metadata(FIRST_UNIQUE_ID)),
        Some(metadata(REUSED_UNIQUE_ID)),
    ]);

    assert!(result.is_none());
}

#[test]
fn disappearance_after_session_lookup_omits_the_candidate() {
    let result = injected_entry([Some(metadata(FIRST_UNIQUE_ID)), None]);

    assert!(result.is_none());
}

#[test]
fn disappearance_before_session_lookup_omits_the_candidate() {
    let result = native_entry_with(TEST_PID, |_| Ok(None), |_| Ok(Some(TEST_SESSION_ID)));

    assert!(result.is_none());
}

#[test]
fn missing_session_omits_the_candidate() {
    let mut first = Some(metadata(FIRST_UNIQUE_ID));
    let result = native_entry_with(TEST_PID, |_| Ok(first.take()), |_| Ok(None));

    assert!(result.is_none());
}

#[test]
fn stable_generation_preserves_the_post_session_metadata() {
    let before = metadata(FIRST_UNIQUE_ID);
    let mut after = before;
    after.parent_pid = TEST_PID;
    after.parent_unique_id = FIRST_UNIQUE_ID;
    let entry = injected_entry([Some(before), Some(after)])
        .expect(STABLE_CANDIDATE_DETAIL)
        .expect(VALID_CANDIDATE_DETAIL);

    assert_eq!(entry.pid, TEST_PID);
    assert_eq!(entry.parent_pid, after.parent_pid);
    assert_eq!(entry.session_id, TEST_SESSION_ID);
    assert_eq!(entry.unique_id, FIRST_UNIQUE_ID);
    assert_eq!(entry.parent_unique_id, after.parent_unique_id);
}

#[test]
fn post_session_metadata_failure_remains_fatal() {
    let mut first = Some(metadata(FIRST_UNIQUE_ID));
    let result = native_entry_with(
        TEST_PID,
        |_| match first.take() {
            Some(metadata) => Ok(Some(metadata)),
            None => Err(std::io::Error::other(INJECTED_METADATA_FAILURE).into()),
        },
        |_| Ok(Some(TEST_SESSION_ID)),
    );

    assert_failure(result, INJECTED_METADATA_FAILURE);
}

#[test]
fn initial_metadata_and_session_failures_remain_fatal() {
    let metadata_failure = native_entry_with(
        TEST_PID,
        |_| Err(std::io::Error::other(INJECTED_INITIAL_METADATA_FAILURE).into()),
        |_| Ok(Some(TEST_SESSION_ID)),
    );
    assert_failure(metadata_failure, INJECTED_INITIAL_METADATA_FAILURE);

    let mut first = Some(metadata(FIRST_UNIQUE_ID));
    let session_failure = native_entry_with(
        TEST_PID,
        |_| Ok(first.take()),
        |_| Err(std::io::Error::other(INJECTED_SESSION_FAILURE).into()),
    );
    assert_failure(session_failure, INJECTED_SESSION_FAILURE);
}

#[test]
fn invalid_process_group_remains_fatal() {
    let mut invalid = metadata(FIRST_UNIQUE_ID);
    invalid.process_group = INVALID_PROCESS_GROUP;
    let result = injected_entry([Some(invalid), Some(invalid)]);

    assert!(result.expect(CANDIDATE_RESULT_DETAIL).is_err());
}

fn injected_entry(
    snapshots: [Option<macos_native::KernelProcessMetadata>; 2],
) -> Option<Result<InventoryEntry>> {
    let mut snapshots = VecDeque::from(snapshots);
    native_entry_with(
        TEST_PID,
        |_| Ok(snapshots.pop_front().expect(METADATA_SNAPSHOT_DETAIL)),
        |_| Ok(Some(TEST_SESSION_ID)),
    )
}

fn metadata(unique_id: u64) -> macos_native::KernelProcessMetadata {
    macos_native::KernelProcessMetadata {
        pid: TEST_PID,
        status: libc::SRUN,
        parent_pid: TEST_PARENT_PID,
        process_group: TEST_PROCESS_GROUP,
        foreground_group: TEST_PROCESS_GROUP,
        start_time: TEST_START_TIME,
        unique_id,
        parent_unique_id: TEST_PARENT_UNIQUE_ID,
        pid_version: TEST_PID_VERSION,
    }
}

fn assert_failure(result: Option<Result<InventoryEntry>>, detail: &str) {
    let error = result.expect(CANDIDATE_RESULT_DETAIL).unwrap_err();
    assert!(error.to_string().contains(detail));
}
