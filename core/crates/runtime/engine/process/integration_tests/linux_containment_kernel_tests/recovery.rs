use super::super::{LinuxContainmentRecovery, LinuxContainmentRecoveryPoll};
use super::record::DurableRecord;
use super::support::{
    SleepingChild, assert_frozen, assert_populated, leaf_path, prepare, wait_empty,
};

const RETAINED_PENDING_POLLS: usize = 3;

#[test]
fn incomplete_record_with_live_migrated_member_freezes_and_remains_pending() {
    let (terminal_id, runtime_epoch, mut lease) = prepare();
    let incomplete = lease.record().clone();
    let record = DurableRecord::write(&incomplete);
    let path = leaf_path(&incomplete);
    let mut child = SleepingChild::spawn();
    let root = child.identity();
    lease
        .migrate_bootstrap(root)
        .expect("migrate member after incomplete record publication");
    drop(lease);

    let mut recovery = LinuxContainmentRecovery::open(incomplete, terminal_id, runtime_epoch, root)
        .expect("open incomplete containment recovery");
    for _ in 0..RETAINED_PENDING_POLLS {
        assert!(matches!(
            recovery.poll().expect("poll incomplete recovery"),
            LinuxContainmentRecoveryPoll::Pending
        ));
        assert_frozen(&path);
        assert_populated(&path, true);
        record.assert_unchanged();
    }

    child.terminate();
    wait_empty(&path);
    assert!(matches!(
        recovery.poll().expect("first stable empty observation"),
        LinuxContainmentRecoveryPoll::Pending
    ));
    assert!(path.is_dir(), "first empty observation must retain custody");
    assert!(matches!(
        recovery.poll().expect("second stable empty observation"),
        LinuxContainmentRecoveryPoll::Settled { leaf_removed: true }
    ));
    assert!(
        !path.exists(),
        "exact empty leaf must be removed before settlement"
    );
    record.assert_unchanged();
    assert!(recovery.into_settlement().is_err());
}
