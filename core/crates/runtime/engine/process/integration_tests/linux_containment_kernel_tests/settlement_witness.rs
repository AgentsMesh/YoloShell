use super::super::constants::{CGROUP_FREEZE_FILE, CGROUP_SUBTREE_CONTROL_FILE};
use super::super::{LinuxContainmentRecovery, LinuxContainmentSettlement};
use super::custody_fixture::prepare_bound_lease;
use super::record::DurableRecord;
use super::settlement_witness_fixture::{observe, settle};
use super::support::{
    SleepingChild, assert_unfrozen, leaf_path, move_member, read_control, wait_empty,
};
use std::fs;

const VERIFY_REPETITIONS: usize = 3;
const IDENTITY_CHANGE: u64 = 1;
const DIFFERENT_BOOT_SUFFIX: &str = "-different";

#[test]
fn removed_witness_outlives_recovery_and_readonly_reopen_preserves_evidence() {
    let (witness, durable) = settle();
    let record = witness.record().clone();
    let controls = read_control(&record.delegated_root, CGROUP_SUBTREE_CONTROL_FILE);
    for _ in 0..VERIFY_REPETITIONS {
        witness.verify().expect("verify retained kernel deletion");
    }
    assert!(!leaf_path(&record).exists());
    durable.assert_unchanged();
    drop(witness);

    let reopened = observe(&record)
        .unwrap()
        .expect("observe already-absent leaf");
    for _ in 0..VERIFY_REPETITIONS {
        reopened.verify().expect("verify same-root absence");
    }
    assert_eq!(reopened.record(), &record);
    assert_eq!(
        read_control(&record.delegated_root, CGROUP_SUBTREE_CONTROL_FILE),
        controls
    );
    assert!(!leaf_path(&record).exists());
    durable.assert_unchanged();
}

#[test]
fn pending_recovery_cannot_yield_a_witness_or_terminate_its_live_member() {
    let (lease, mut child, _pty) = prepare_bound_lease();
    let record = lease.record().clone();
    let durable = DurableRecord::write(&record);
    let path = leaf_path(&record);
    let recovery = LinuxContainmentRecovery::open(
        record.clone(),
        record.terminal_id,
        record.runtime_epoch,
        child.identity(),
    )
    .unwrap();
    assert!(recovery.into_settlement().is_err());
    assert!(child.identity().matches());
    assert_unfrozen(&path);
    assert!(observe(&record).unwrap().is_none());
    assert_unfrozen(&path);
    assert!(child.identity().matches());
    durable.assert_unchanged();
    child.terminate();
    wait_empty(&path);
    lease.remove_empty().unwrap();
}

#[test]
fn an_empty_present_leaf_is_readonly_pending_until_kernel_retirement() {
    let (lease, mut child, _pty) = prepare_bound_lease();
    let record = lease.record().clone();
    let path = leaf_path(&record);
    child.terminate();
    wait_empty(&path);
    assert_unfrozen(&path);
    assert!(observe(&record).unwrap().is_none());
    assert_unfrozen(&path);
    lease.remove_empty().unwrap();
    observe(&record)
        .unwrap()
        .expect("removed exact leaf is absent")
        .verify()
        .unwrap();
}

#[test]
fn same_name_replacement_is_rejected_without_freezing_or_signaling_it() {
    let (witness, durable) = settle();
    let record = witness.record().clone();
    let path = leaf_path(&record);
    fs::create_dir(&path).expect("create replacement cgroup");
    let mut replacement = SleepingChild::spawn();
    move_member(&path, replacement.identity());
    let freeze = read_control(&path, CGROUP_FREEZE_FILE);
    assert!(witness.verify().is_err());
    assert!(observe(&record).is_err());
    assert!(replacement.identity().matches());
    assert_eq!(read_control(&path, CGROUP_FREEZE_FILE), freeze);
    assert_unfrozen(&path);
    durable.assert_unchanged();
    replacement.terminate();
    wait_empty(&path);
    fs::remove_dir(&path).expect("remove validated replacement after test");
    witness.verify().unwrap();
}

#[test]
fn a_retained_empty_leaf_is_not_kernel_deletion_evidence() {
    let (lease, mut child, _pty) = prepare_bound_lease();
    let record = lease.record().clone();
    let path = leaf_path(&record);
    let candidate = LinuxContainmentSettlement::capture(
        &lease.root,
        Some(&lease.leaf),
        record,
        child.identity(),
    )
    .unwrap();
    child.terminate();
    wait_empty(&path);
    assert!(candidate.verify().is_err());
    assert!(path.is_dir());
    assert_unfrozen(&path);
    lease.remove_empty().unwrap();
    candidate.verify().unwrap();
}

#[test]
fn readonly_reopen_rejects_changed_root_boot_and_runtime_binding() {
    let (witness, durable) = settle();
    let record = witness.record().clone();
    let mut replaced = record.clone();
    replaced.root_identity.inode = replaced.root_identity.inode.wrapping_add(IDENTITY_CHANGE);
    assert!(observe(&replaced).is_err());
    replaced = record.clone();
    replaced.boot_id.push_str(DIFFERENT_BOOT_SUFFIX);
    assert!(observe(&replaced).is_err());
    replaced = record.clone();
    replaced.leaf_identity.mount_id = replaced
        .root_identity
        .mount_id
        .wrapping_add(IDENTITY_CHANGE);
    assert!(observe(&replaced).is_err());
    replaced = record.clone();
    replaced.pty_device = None;
    replaced.pty_capability = None;
    assert!(observe(&replaced).is_err());
    let root = crate::process::ProcessIdentity {
        pid: record.bound_pid.unwrap(),
        start_time: record.bound_start_time.unwrap(),
    };
    assert!(
        LinuxContainmentSettlement::observe_settled(
            record.clone(),
            uuid::Uuid::new_v4(),
            record.runtime_epoch,
            root,
        )
        .is_err()
    );
    assert!(
        LinuxContainmentSettlement::observe_settled(
            record.clone(),
            record.terminal_id,
            uuid::Uuid::new_v4(),
            root,
        )
        .is_err()
    );
    assert!(
        LinuxContainmentSettlement::observe_settled(
            record.clone(),
            record.terminal_id,
            record.runtime_epoch,
            crate::process::ProcessIdentity {
                start_time: root.start_time.wrapping_add(IDENTITY_CHANGE),
                ..root
            },
        )
        .is_err()
    );
    witness.verify().unwrap();
    durable.assert_unchanged();
}
