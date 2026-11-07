use super::super::constants::CGROUP_FREEZE_FILE;
use super::super::{
    LinuxContainmentError, LinuxContainmentRecovery, LinuxContainmentRecoveryPoll,
    LinuxContainmentSettlement, descriptor,
};
use super::custody_fixture::prepare_bound_lease;
use super::record::DurableRecord;
use super::settlement_witness_fixture::{observe, settle};
use super::support::{
    SleepingChild, assert_unfrozen, leaf_path, move_member, read_control, wait_empty,
};
use crate::process::ProcessIdentity;
use std::fs::Permissions;
use std::io::ErrorKind;
use std::os::unix::fs::PermissionsExt;

const NO_ACCESS_MODE: u32 = 0;

#[test]
fn restarted_recovery_preserves_complete_already_absent_retirement() {
    let (witness, durable) = settle();
    let record = witness.record().clone();
    let root = ProcessIdentity {
        pid: record.bound_pid.unwrap(),
        start_time: record.bound_start_time.unwrap(),
    };
    drop(witness);

    let mut recovery = LinuxContainmentRecovery::open(
        record.clone(),
        record.terminal_id,
        record.runtime_epoch,
        root,
    )
    .unwrap();
    assert!(matches!(
        recovery.poll().unwrap(),
        LinuxContainmentRecoveryPoll::Settled { leaf_removed: true }
    ));
    let reopened = recovery.into_settlement().unwrap();
    reopened.verify().unwrap();
    assert_eq!(reopened.record(), &record);
    assert!(!leaf_path(&record).exists());
    durable.assert_unchanged();
}

#[test]
fn recorded_root_cannot_redirect_recovery_outside_the_configured_delegation() {
    let (witness, durable) = settle();
    let mut redirected = witness.record().clone();
    let other_root = redirected.delegated_root.parent().unwrap().to_path_buf();
    let other = descriptor::open_root(&other_root).unwrap();
    redirected.delegated_root = other_root;
    redirected.root_identity = descriptor::identity(&other).unwrap();

    assert!(matches!(
        observe(&redirected),
        Err(LinuxContainmentError::Identity(_))
    ));
    witness.verify().unwrap();
    durable.assert_unchanged();
}

#[test]
fn inaccessible_replacement_is_not_absence_and_keeps_its_live_member_untouched() {
    let (witness, durable) = settle();
    let record = witness.record().clone();
    let path = leaf_path(&record);
    let root = descriptor::open_root(&record.delegated_root).unwrap();
    let replacement = descriptor::create_leaf(&root, &record.leaf_name).unwrap();
    let mut child = SleepingChild::spawn();
    move_member(&path, child.identity());
    let freeze_before = read_control(&path, CGROUP_FREEZE_FILE);
    let original = replacement.metadata().unwrap().permissions();
    replacement
        .set_permissions(Permissions::from_mode(NO_ACCESS_MODE))
        .unwrap();

    let observed = observe(&record);
    let verified = witness.verify();

    replacement.set_permissions(original).unwrap();
    let freeze_after = read_control(&path, CGROUP_FREEZE_FILE);
    let survived = child.identity().matches();
    child.terminate();
    wait_empty(&path);
    descriptor::remove_exact_leaf(&root, &replacement, &record.leaf_name).unwrap();
    assert!(matches!(observed,
        Err(LinuxContainmentError::Io(error)) if error.kind() == ErrorKind::PermissionDenied
    ));
    assert!(matches!(verified,
        Err(LinuxContainmentError::Io(error)) if error.kind() == ErrorKind::PermissionDenied
    ));
    assert!(survived);
    assert_eq!(freeze_after, freeze_before);
    witness.verify().unwrap();
    durable.assert_unchanged();
}

#[test]
fn retained_leaf_cannot_be_substituted_for_delegated_root_authority() {
    let (lease, mut child, _pty) = prepare_bound_lease();
    let record = lease.record().clone();
    let path = leaf_path(&record);
    let durable = DurableRecord::write(&record);
    let wrong_root = LinuxContainmentSettlement::capture(
        &lease.leaf,
        Some(&lease.leaf),
        record,
        child.identity(),
    )
    .unwrap();

    let rejected = matches!(wrong_root.verify(), Err(LinuxContainmentError::Identity(_)));
    let survived = child.identity().matches();
    assert_unfrozen(&path);
    child.terminate();
    wait_empty(&path);
    lease.remove_empty().unwrap();
    assert!(rejected);
    assert!(survived);
    durable.assert_unchanged();
}
