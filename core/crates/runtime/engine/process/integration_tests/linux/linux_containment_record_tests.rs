use super::constants::{RECORD_SCHEMA_VERSION, session_leaf_name};
use super::{CgroupDirectoryIdentity, LinuxContainmentError, LinuxContainmentRecord};
use crate::process::ProcessIdentity;
use std::path::PathBuf;
use uuid::Uuid;

const TEST_BOOT_ID: &str = "linux-boot";

#[test]
fn durable_identity_rejects_runtime_boot_and_token_mismatch() {
    let terminal_id = Uuid::new_v4();
    let runtime_epoch = Uuid::new_v4();
    let mut record = fixture(terminal_id, runtime_epoch);
    assert!(
        record
            .validate(terminal_id, runtime_epoch, TEST_BOOT_ID)
            .is_ok()
    );
    assert!(matches!(
        record.validate(Uuid::new_v4(), runtime_epoch, TEST_BOOT_ID),
        Err(LinuxContainmentError::Identity(_))
    ));
    record.boot_id = "another-boot".to_owned();
    assert!(
        record
            .validate(terminal_id, runtime_epoch, TEST_BOOT_ID)
            .is_err()
    );
    record.boot_id = TEST_BOOT_ID.to_owned();
    record.leaf_name = "yolo-session-forged".to_owned();
    assert!(
        record
            .validate(terminal_id, runtime_epoch, TEST_BOOT_ID)
            .is_err()
    );
    record.leaf_name = session_leaf_name(terminal_id, runtime_epoch);
    record.lease_token = Uuid::nil();
    assert!(
        record
            .validate(terminal_id, runtime_epoch, TEST_BOOT_ID)
            .is_err()
    );
}

#[test]
fn durable_identity_rejects_unknown_boot_and_invalid_leaf() {
    let terminal_id = Uuid::new_v4();
    let runtime_epoch = Uuid::new_v4();
    let mut record = fixture(terminal_id, runtime_epoch);
    record.boot_id = "unknown:test".to_owned();
    assert!(
        record
            .validate(terminal_id, runtime_epoch, "unknown:test")
            .is_err()
    );
    record.boot_id = TEST_BOOT_ID.to_owned();
    record.leaf_name = "../escape".to_owned();
    assert!(
        record
            .validate(terminal_id, runtime_epoch, TEST_BOOT_ID)
            .is_err()
    );
}

#[test]
fn recovery_requires_complete_pty_and_bound_root_evidence() {
    let terminal_id = Uuid::new_v4();
    let runtime_epoch = Uuid::new_v4();
    let root = ProcessIdentity {
        pid: 41,
        start_time: 73,
    };
    let mut record = fixture(terminal_id, runtime_epoch);
    assert!(
        record
            .validate_recovery(terminal_id, runtime_epoch, TEST_BOOT_ID, root)
            .is_ok_and(|complete| !complete)
    );
    record.pty_device = Some(19);
    record.pty_capability = Some("custody-token".to_owned());
    record.bound_pid = Some(root.pid);
    record.bound_start_time = Some(root.start_time);
    assert!(
        record
            .validate_recovery(terminal_id, runtime_epoch, TEST_BOOT_ID, root)
            .is_ok_and(|complete| complete)
    );
    let reused = ProcessIdentity {
        start_time: root.start_time + 1,
        ..root
    };
    assert!(
        record
            .validate_recovery(terminal_id, runtime_epoch, TEST_BOOT_ID, reused)
            .is_err()
    );
}

fn fixture(terminal_id: Uuid, runtime_epoch: Uuid) -> LinuxContainmentRecord {
    LinuxContainmentRecord {
        schema_version: RECORD_SCHEMA_VERSION,
        terminal_id,
        runtime_epoch,
        boot_id: TEST_BOOT_ID.to_owned(),
        delegated_root: PathBuf::from("/sys/fs/cgroup/yolo"),
        root_identity: CgroupDirectoryIdentity {
            mount_id: 1,
            inode: 2,
        },
        leaf_name: session_leaf_name(terminal_id, runtime_epoch),
        leaf_identity: CgroupDirectoryIdentity {
            mount_id: 1,
            inode: 3,
        },
        lease_token: Uuid::new_v4(),
        pty_device: None,
        pty_capability: None,
        bound_pid: None,
        bound_start_time: None,
    }
}
