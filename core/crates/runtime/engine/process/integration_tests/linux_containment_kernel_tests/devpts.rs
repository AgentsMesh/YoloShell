use super::devpts_assertions::{assert_no_bytes, read_payload};
use super::devpts_fixture::{ForkWriter, PtyPair, STALE_PAYLOAD, allocate_reused};
use super::support::{
    THAW_VALUE, assert_frozen, freeze_members, leaf_path, prepare, wait_empty, write_freeze,
};

#[test]
fn unrelated_exec_child_does_not_retain_fixture_pty_descriptors() {
    let old = PtyPair::open();
    let old_path = old.path.clone();
    let old_device = old.device();
    let mut child = super::support::SleepingChild::spawn();
    drop(old);
    let (reused, _retained) = allocate_reused(&old_path);
    assert_eq!(reused.device(), old_device);
    assert!(child.identity().matches());
    child.terminate();
}

#[test]
fn reused_devpts_number_cannot_receive_from_a_frozen_stale_writer() {
    let (_session_id, _runtime_epoch, mut lease) = prepare();
    let old = PtyPair::open();
    let old_path = old.path.clone();
    let old_device = old.device();
    let mut writer = ForkWriter::spawn(&old);
    lease
        .migrate_bootstrap(writer.identity())
        .expect("migrate stale writer");
    let leaf = leaf_path(lease.record());
    assert_eq!(freeze_members(&mut lease).len(), 1);
    assert_frozen(&leaf);
    drop(old);

    let (mut reused, _retained) = allocate_reused(&old_path);
    assert_eq!(reused.device(), old_device);
    reused.make_master_nonblocking();
    writer.release();
    assert_no_bytes(&mut reused.master);
    assert_frozen(&leaf);

    write_freeze(&leaf, THAW_VALUE);
    assert_eq!(read_payload(&mut reused.master), STALE_PAYLOAD);
    writer.wait();
    wait_empty(&leaf);
    lease
        .remove_empty()
        .expect("remove drained stale-writer cgroup");
}
