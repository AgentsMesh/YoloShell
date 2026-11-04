use super::*;
use std::cell::Cell;

#[path = "macos_lineage_fault_tests.rs"]
mod fault_tests;

const CHILD_PID: u32 = 101;
const PARENT_PID: u32 = 100;
const CHILD_UNIQUE_ID: u64 = 201;
const PARENT_UNIQUE_ID: u64 = 200;

fn short(parent: u32) -> libc::proc_bsdshortinfo {
    let mut value: libc::proc_bsdshortinfo = unsafe { std::mem::zeroed() };
    value.pbsi_pid = CHILD_PID;
    value.pbsi_ppid = parent;
    value.pbsi_status = libc::SRUN;
    value
}

fn unique(child: u64, parent: u64) -> ProcUniqueIdentifierInfo {
    let mut value: ProcUniqueIdentifierInfo = unsafe { std::mem::zeroed() };
    value.unique_id = child;
    value.parent_unique_id = parent;
    value
}

#[test]
fn stable_lineage_is_retained() {
    let observed = capture_with(
        CHILD_PID,
        || Ok(Some(short(PARENT_PID))),
        || Ok(Some(unique(CHILD_UNIQUE_ID, PARENT_UNIQUE_ID))),
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        observed,
        ProcessLineage {
            pid: CHILD_PID,
            parent_pid: PARENT_PID,
            unique_id: CHILD_UNIQUE_ID,
            parent_unique_id: PARENT_UNIQUE_ID,
        }
    );
}

#[test]
fn reparenting_and_pid_reuse_are_rejected() {
    for changed_parent in [false, true] {
        let reads = Cell::new(0);
        let result = capture_with(
            CHILD_PID,
            || Ok(Some(short(PARENT_PID))),
            || {
                let count = reads.get();
                reads.set(count + 1);
                let changed = u64::from(count > 0);
                Ok(Some(unique(
                    CHILD_UNIQUE_ID + if changed_parent { 0 } else { changed },
                    PARENT_UNIQUE_ID + if changed_parent { changed } else { 0 },
                )))
            },
        );
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains(CHANGED_IDENTITY_DETAIL)
        );
    }
    let reads = Cell::new(0);
    assert!(
        capture_with(
            CHILD_PID,
            || {
                let count = reads.get();
                reads.set(count + 1);
                Ok(Some(short(PARENT_PID + count)))
            },
            || Ok(Some(unique(CHILD_UNIQUE_ID, PARENT_UNIQUE_ID)))
        )
        .is_err()
    );
}

#[test]
fn missing_or_zombie_process_is_absent_but_denied_metadata_is_not() {
    assert!(
        capture_with(
            CHILD_PID,
            || Ok(None),
            || Ok(Some(unique(CHILD_UNIQUE_ID, PARENT_UNIQUE_ID)))
        )
        .unwrap()
        .is_none()
    );
    assert!(
        capture_with(CHILD_PID, || Ok(Some(short(PARENT_PID))), || Ok(None))
            .unwrap()
            .is_none()
    );
    assert!(
        capture_with(
            CHILD_PID,
            || {
                let mut value = short(PARENT_PID);
                value.pbsi_status = libc::SZOMB;
                Ok(Some(value))
            },
            || Ok(Some(unique(CHILD_UNIQUE_ID, PARENT_UNIQUE_ID)))
        )
        .unwrap()
        .is_none()
    );
    let error = capture_with(
        CHILD_PID,
        || Err(std::io::Error::from_raw_os_error(libc::EPERM).into()),
        || Ok(Some(unique(CHILD_UNIQUE_ID, PARENT_UNIQUE_ID))),
    )
    .unwrap_err();
    assert_eq!(
        error
            .downcast_ref::<std::io::Error>()
            .unwrap()
            .raw_os_error(),
        Some(libc::EPERM)
    );
}
