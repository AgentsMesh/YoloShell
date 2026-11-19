use super::super::constants::{CGROUP_FREEZE_FILE, FREEZE_TIMEOUT, FREEZE_VALUE};
use super::super::{LinuxContainmentError, LinuxContainmentPreparation, descriptor};
use super::record::DurableRecord;
use super::support::{
    SleepingChild, THAW_VALUE, assert_frozen, assert_unfrozen, freeze_members, leaf_identity,
    leaf_path, move_member, prepare, read_control, wait_empty, wait_member_count, write_freeze,
};
use std::collections::HashSet;
use std::time::Instant;

const FREEZE_TIMEOUT_DETAIL: &str = "session cgroup did not freeze before its deadline";

#[test]
fn armed_freeze_loss_rewrites_one_without_reset_and_timeout_preserves_custody() {
    let (_session_id, _runtime_epoch, mut lease) = prepare();
    let mut child = SleepingChild::spawn();
    lease
        .migrate_bootstrap(child.identity())
        .expect("migrate member");
    let record = DurableRecord::write(lease.record());
    let path = leaf_path(lease.record());
    let identity = leaf_identity(&lease);
    assert_eq!(freeze_members(&mut lease).len(), 1);
    let armed_at = lease.freeze_requested_at.expect("freeze latch must arm");

    write_freeze(&path, THAW_VALUE);
    assert_unfrozen(&path);
    assert!(
        lease
            .poll_frozen_members(false)
            .expect("re-arm lost freeze")
            .is_none()
    );
    assert_eq!(lease.freeze_requested_at, Some(armed_at));
    assert_eq!(
        read_control(&path, CGROUP_FREEZE_FILE).trim(),
        FREEZE_VALUE.trim()
    );
    assert_frozen(&path);

    let expired_at = Instant::now() - FREEZE_TIMEOUT;
    lease.freeze_requested_at = Some(expired_at);
    write_freeze(&path, THAW_VALUE);
    assert_unfrozen(&path);
    let error = lease
        .poll_frozen_members(false)
        .err()
        .expect("expired armed freeze must fail closed");
    assert!(matches!(
        error,
        LinuxContainmentError::Unavailable(detail) if detail == FREEZE_TIMEOUT_DETAIL
    ));
    assert_eq!(lease.freeze_requested_at, Some(expired_at));
    assert_eq!(
        read_control(&path, CGROUP_FREEZE_FILE).trim(),
        FREEZE_VALUE.trim()
    );
    assert_frozen(&path);
    assert_eq!(leaf_identity(&lease), identity);
    record.assert_unchanged();

    child.terminate();
    wait_empty(&path);
    lease.remove_empty().expect("remove drained cgroup");
}

#[test]
fn retained_membership_insertion_and_removal_never_thaw_or_retire_early() {
    let (_session_id, _runtime_epoch, mut lease) = prepare();
    let mut original = SleepingChild::spawn();
    lease
        .migrate_bootstrap(original.identity())
        .expect("migrate original survivor");
    let record = DurableRecord::write(lease.record());
    let path = leaf_path(lease.record());
    let identity = leaf_identity(&lease);
    let original_members = freeze_members(&mut lease);
    let LinuxContainmentPreparation::Retained(ready) = lease
        .prepare_cleanup(&original_members)
        .expect("prepare retained survivor")
    else {
        panic!("nonempty frozen cgroup must be retained");
    };
    assert!(
        lease
            .validate_retained(&ready)
            .expect("validate retained survivor")
    );

    let mut inserted = SleepingChild::spawn();
    move_member(&path, inserted.identity());
    wait_member_count(&path, 2);
    assert!(
        !lease
            .validate_retained(&ready)
            .expect("reject inserted member")
    );
    assert!(matches!(
        lease
            .prepare_cleanup(&original_members)
            .expect("observe insertion"),
        LinuxContainmentPreparation::MembershipChanged
    ));
    assert_eq!(leaf_identity(&lease), identity);
    assert_frozen(&path);
    record.assert_unchanged();

    inserted.terminate();
    wait_member_count(&path, 1);
    assert!(
        lease
            .validate_retained(&ready)
            .expect("revalidate original")
    );
    original.terminate();
    wait_empty(&path);
    assert!(
        !lease
            .validate_retained(&ready)
            .expect("reject removed survivor")
    );
    assert!(matches!(
        lease
            .prepare_cleanup(&HashSet::new())
            .expect("remove exact empty leaf"),
        LinuxContainmentPreparation::Removed
    ));
    assert!(!path.exists());
    record.assert_unchanged();
}

#[test]
fn exact_remove_rejects_same_name_replacement_without_deleting_it() {
    let (_session_id, _runtime_epoch, mut lease) = prepare();
    let path = leaf_path(lease.record());
    let record = DurableRecord::write(lease.record());
    assert!(freeze_members(&mut lease).is_empty());
    let old_identity = leaf_identity(&lease);
    std::fs::remove_dir(&path).expect("remove original empty cgroup");
    std::fs::create_dir(&path).expect("create same-name replacement cgroup");
    let replacement =
        descriptor::open_leaf(&lease.root, lease.leaf_name()).expect("open replacement cgroup");
    let replacement_identity = descriptor::identity(&replacement).expect("identify replacement");
    assert_ne!(replacement_identity, old_identity);

    assert!(descriptor::remove_exact_leaf(&lease.root, &lease.leaf, lease.leaf_name()).is_err());
    let current = descriptor::open_leaf(&lease.root, lease.leaf_name())
        .expect("replacement must remain present");
    assert_eq!(
        descriptor::identity(&current).unwrap(),
        replacement_identity
    );
    record.assert_unchanged();
    drop(current);
    drop(replacement);
    drop(lease);
    std::fs::remove_dir(&path).expect("remove replacement after proof");
}

#[test]
fn unstable_settlement_retries_without_thawing_or_discarding_the_custody_leaf() {
    let (_session_id, _runtime_epoch, mut lease) = prepare();
    let mut child = SleepingChild::spawn();
    lease.migrate_bootstrap(child.identity()).unwrap();
    let members = freeze_members(&mut lease);
    let path = leaf_path(lease.record());
    let original = leaf_identity(&lease);
    let LinuxContainmentPreparation::Retained(ready) = lease.prepare_cleanup(&members).unwrap()
    else {
        panic!("live member must retain custody");
    };
    write_freeze(&path, THAW_VALUE);
    assert_unfrozen(&path);
    assert!(matches!(
        lease.prepare_cleanup(&members).unwrap(),
        LinuxContainmentPreparation::MembershipChanged
    ));
    assert_eq!(leaf_identity(&lease), original);
    assert_frozen(&path);
    write_freeze(&path, THAW_VALUE);
    assert_unfrozen(&path);
    assert!(!lease.validate_retained(&ready).unwrap());
    assert_eq!(leaf_identity(&lease), original);
    assert_frozen(&path);
    child.terminate();
    wait_empty(&path);
    lease.remove_empty().unwrap();
}
