use super::*;
use std::cell::Cell;

const TEST_PID: u32 = 41;
const TEST_PARENT_PID: u32 = 37;
const TEST_PROCESS_GROUP: i32 = 31;
const TEST_SESSION_ID: u32 = 29;
const TEST_START_TIME: u64 = 23;
const TEST_UNIQUE_ID: u64 = 19;
const REUSED_UNIQUE_ID: u64 = 17;
const TEST_PARENT_UNIQUE_ID: u64 = 13;
const TEST_PID_VERSION: u32 = 11;
const FIRST_CALL: usize = 0;
const FINAL_CALL: usize = 1;
const CALL_INCREMENT: usize = 1;
const ACTION_FAILURE: &str = "injected inventory action failure";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    Stable,
    InitialMissing,
    InitialFailure,
    SessionMissing,
    SessionFailure,
    FinalMissing,
    FinalFailure,
    ReusedIdentity,
    InvalidGroup,
}

#[cfg(feature = "yolo-unit-tests")]
#[test]
fn one_native_entry_instance_covers_stability_failure_and_disappearance() {
    for action in [
        Action::Stable,
        Action::InitialMissing,
        Action::InitialFailure,
        Action::SessionMissing,
        Action::SessionFailure,
        Action::FinalMissing,
        Action::FinalFailure,
        Action::ReusedIdentity,
        Action::InvalidGroup,
    ] {
        let metadata_calls = Cell::new(FIRST_CALL);
        let result = native_entry_with(
            TEST_PID,
            |_| {
                let call = metadata_calls.get();
                metadata_calls.set(call + CALL_INCREMENT);
                match (action, call) {
                    (Action::InitialMissing, FIRST_CALL) | (Action::FinalMissing, FINAL_CALL) => {
                        Ok(None)
                    }
                    (Action::InitialFailure, FIRST_CALL) | (Action::FinalFailure, FINAL_CALL) => {
                        Err(std::io::Error::other(ACTION_FAILURE).into())
                    }
                    (Action::ReusedIdentity, FINAL_CALL) => Ok(Some(metadata(REUSED_UNIQUE_ID))),
                    (Action::InvalidGroup, _) => {
                        let mut value = metadata(TEST_UNIQUE_ID);
                        value.process_group = i32::MIN;
                        Ok(Some(value))
                    }
                    _ => Ok(Some(metadata(TEST_UNIQUE_ID))),
                }
            },
            |_| match action {
                Action::SessionMissing => Ok(None),
                Action::SessionFailure => Err(std::io::Error::other(ACTION_FAILURE).into()),
                _ => Ok(Some(TEST_SESSION_ID)),
            },
        );

        match action {
            Action::Stable => assert!(matches!(result, Some(Ok(_)))),
            Action::InitialFailure
            | Action::SessionFailure
            | Action::FinalFailure
            | Action::InvalidGroup => assert!(matches!(result, Some(Err(_)))),
            _ => assert!(result.is_none(), "action {action:?} retained a candidate"),
        }
    }
}

#[cfg(feature = "yolo-unit-tests")]
#[test]
fn process_session_lookup_distinguishes_live_missing_and_unrepresentable_ids() {
    assert!(process_session_id(std::process::id()).unwrap().is_some());
    assert_eq!(process_session_id(i32::MAX as u32).unwrap(), None);
    assert!(process_session_id(u32::MAX).is_err());
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
