use super::*;
use std::cell::Cell;

const MATRIX_PID: u32 = 97;
const MATRIX_PARENT_PID: u32 = 89;
const MATRIX_PROCESS_GROUP: i32 = 83;
const MATRIX_SESSION_ID: u32 = 79;
const MATRIX_START_TIME: u64 = 73;
const MATRIX_UNIQUE_ID: u64 = 71;
const MATRIX_REUSED_UNIQUE_ID: u64 = 67;
const MATRIX_PARENT_UNIQUE_ID: u64 = 61;
const MATRIX_PID_VERSION: u32 = 59;
const FIRST_METADATA_CALL: u8 = 0;
const METADATA_CALL_INCREMENT: u8 = 1;
const MATRIX_FAILURE_DETAIL: &str = "injected inventory matrix failure";

#[derive(Clone, Copy, Debug)]
enum MatrixCase {
    BeforeMissing,
    BeforeFailure,
    SessionMissing,
    SessionFailure,
    AfterMissing,
    AfterFailure,
    Reused,
    InvalidProcessGroup,
    Stable,
}

#[test]
fn one_native_entry_instance_covers_every_fencing_outcome() {
    for case in [
        MatrixCase::BeforeMissing,
        MatrixCase::BeforeFailure,
        MatrixCase::SessionMissing,
        MatrixCase::SessionFailure,
        MatrixCase::AfterMissing,
        MatrixCase::AfterFailure,
        MatrixCase::Reused,
        MatrixCase::InvalidProcessGroup,
        MatrixCase::Stable,
    ] {
        assert_matrix_case(case);
    }
}

fn assert_matrix_case(case: MatrixCase) {
    let calls = Cell::new(FIRST_METADATA_CALL);
    let result = native_entry_with(
        MATRIX_PID,
        |_| {
            let call = calls.get();
            calls.set(call.saturating_add(METADATA_CALL_INCREMENT));
            match (case, call) {
                (MatrixCase::BeforeMissing, FIRST_METADATA_CALL)
                | (MatrixCase::AfterMissing, METADATA_CALL_INCREMENT) => Ok(None),
                (MatrixCase::BeforeFailure, FIRST_METADATA_CALL)
                | (MatrixCase::AfterFailure, METADATA_CALL_INCREMENT) => {
                    Err(anyhow::anyhow!(MATRIX_FAILURE_DETAIL))
                }
                (MatrixCase::Reused, METADATA_CALL_INCREMENT) => Ok(Some(metadata(
                    MATRIX_REUSED_UNIQUE_ID,
                    MATRIX_PROCESS_GROUP,
                ))),
                (MatrixCase::InvalidProcessGroup, _) => Ok(Some(metadata(MATRIX_UNIQUE_ID, -1))),
                _ => Ok(Some(metadata(MATRIX_UNIQUE_ID, MATRIX_PROCESS_GROUP))),
            }
        },
        |_| match case {
            MatrixCase::SessionMissing => Ok(None),
            MatrixCase::SessionFailure => Err(anyhow::anyhow!(MATRIX_FAILURE_DETAIL)),
            _ => Ok(Some(MATRIX_SESSION_ID)),
        },
    );

    match case {
        MatrixCase::Stable => {
            let entry = result.unwrap().unwrap();
            assert_eq!(entry.session_id, MATRIX_SESSION_ID);
            assert_eq!(entry.unique_id, MATRIX_UNIQUE_ID);
        }
        MatrixCase::BeforeFailure | MatrixCase::SessionFailure | MatrixCase::AfterFailure => {
            assert!(
                result
                    .unwrap()
                    .unwrap_err()
                    .to_string()
                    .contains(MATRIX_FAILURE_DETAIL)
            );
        }
        MatrixCase::InvalidProcessGroup => assert!(result.unwrap().is_err()),
        MatrixCase::BeforeMissing
        | MatrixCase::SessionMissing
        | MatrixCase::AfterMissing
        | MatrixCase::Reused => assert!(result.is_none()),
    }
}

#[test]
fn native_session_lookup_covers_live_absent_and_unrepresentable_pids() {
    assert!(process_session_id(std::process::id()).unwrap().is_some());
    assert!(process_session_id(i32::MAX as u32).unwrap().is_none());
    assert!(process_session_id(u32::MAX).is_err());
}

fn metadata(unique_id: u64, process_group: i32) -> macos_native::KernelProcessMetadata {
    macos_native::KernelProcessMetadata {
        pid: MATRIX_PID,
        status: libc::SRUN,
        parent_pid: MATRIX_PARENT_PID,
        process_group,
        foreground_group: MATRIX_PROCESS_GROUP,
        start_time: MATRIX_START_TIME,
        unique_id,
        parent_unique_id: MATRIX_PARENT_UNIQUE_ID,
        pid_version: MATRIX_PID_VERSION,
    }
}
