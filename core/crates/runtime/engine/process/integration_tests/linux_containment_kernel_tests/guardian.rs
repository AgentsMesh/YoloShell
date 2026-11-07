use super::devpts_assertions::{assert_missing_eof, assert_no_bytes, read_payload};
use super::devpts_fixture::{ForkWriter, PtyPair, STALE_PAYLOAD};
use super::guardian_fixture::{ExternalGuardian, MasterOwningWorker};
use super::guardian_race::{ReuseRace, ReuseResult};
use super::record::DurableRecord;
use super::support::{
    SleepingChild, THAW_VALUE, assert_frozen, assert_unfrozen, leaf_identity, leaf_path,
    move_member, prepare, wait_empty, wait_member_count, write_freeze,
};
use std::os::unix::fs::MetadataExt;

const MINIMUM_RACE_ATTEMPTS: usize = 2;
const WRITER_MEMBER_COUNT: usize = 1;
const WRITER_AND_SURVIVOR_COUNT: usize = 2;

#[test]
fn worker_sigkill_eof_auto_arms_guardian_before_exact_leaf_release() {
    let (_session_id, _runtime_epoch, mut lease) = prepare();
    let old = PtyPair::open();
    let old_path = old.path.clone();
    let old_device = old.device();
    let mut worker = MasterOwningWorker::spawn(&old);
    let mut writer = ForkWriter::spawn(&old);
    lease
        .migrate_bootstrap(writer.identity())
        .expect("migrate HUP-ignoring stale writer");
    let record = DurableRecord::write(lease.record());
    let leaf = leaf_path(lease.record());
    let cgroup_identity = leaf_identity(&lease);
    let mut guardian = ExternalGuardian::spawn(&old, &lease, &writer, &mut worker);

    drop(old);
    assert!(writer.identity().matches());
    worker.kill();
    guardian.await_worker_loss();
    assert!(writer.identity().matches());
    assert_frozen(&leaf);
    assert_old_object(&old_path, old_device);
    let race = ReuseRace::start(&old_path);

    write_freeze(&leaf, THAW_VALUE);
    assert_unfrozen(&leaf);
    guardian.release_alias();
    writer.release();
    let ReuseResult {
        mut reused,
        retained: _retained,
        attempts,
    } = race.finish();
    assert!(attempts >= MINIMUM_RACE_ATTEMPTS);
    assert_eq!(reused.device(), old_device);
    reused.make_master_nonblocking();
    assert_no_bytes(&mut reused.master);
    assert_frozen(&leaf);
    assert_eq!(leaf_identity(&lease), cgroup_identity);
    assert_eq!(
        super::support::read_control(&leaf, super::super::constants::CGROUP_PROCESSES_FILE)
            .lines()
            .count(),
        WRITER_MEMBER_COUNT
    );
    record.assert_unchanged();

    write_freeze(&leaf, THAW_VALUE);
    assert_eq!(read_payload(&mut reused.master), STALE_PAYLOAD);
    writer.wait();
    wait_empty(&leaf);
    lease
        .remove_empty()
        .expect("remove drained guardian cgroup");
}

#[test]
fn master_close_pending_guardian_death_keeps_missing_eof_survivor_frozen() {
    let (_session_id, _runtime_epoch, mut lease) = prepare();
    let old = PtyPair::open();
    old.make_master_nonblocking();
    let old_path = old.path.clone();
    let old_device = old.device();
    let mut worker = MasterOwningWorker::spawn(&old);
    let mut writer = ForkWriter::spawn(&old);
    lease
        .migrate_bootstrap(writer.identity())
        .expect("migrate stale writer");
    let leaf = leaf_path(lease.record());
    let mut survivor = SleepingChild::spawn();
    move_member(&leaf, survivor.identity());
    wait_member_count(&leaf, WRITER_AND_SURVIVOR_COUNT);
    let record = DurableRecord::write(lease.record());
    let cgroup_identity = leaf_identity(&lease);
    let mut guardian = ExternalGuardian::spawn(&old, &lease, &writer, &mut worker);

    let mut missing_eof_reader = old
        .master
        .try_clone()
        .expect("clone parent-only PTY EOF probe");
    guardian.arm_close_pending();
    assert_frozen(&leaf);
    worker.kill();
    guardian.await_worker_loss();
    assert!(writer.identity().matches());
    assert_missing_eof(&mut missing_eof_reader);
    assert_old_object(&old_path, old_device);
    let race = ReuseRace::start(&old_path);
    drop(missing_eof_reader);
    drop(old);

    guardian.kill();
    writer.release();
    let ReuseResult {
        mut reused,
        retained: _retained,
        attempts,
    } = race.finish();
    assert!(attempts >= MINIMUM_RACE_ATTEMPTS);
    assert_eq!(reused.device(), old_device);
    reused.make_master_nonblocking();
    assert_no_bytes(&mut reused.master);
    assert_frozen(&leaf);
    wait_member_count(&leaf, WRITER_AND_SURVIVOR_COUNT);
    assert_eq!(leaf_identity(&lease), cgroup_identity);
    record.assert_unchanged();

    drop(writer);
    wait_member_count(&leaf, WRITER_MEMBER_COUNT);
    survivor.terminate();
    wait_empty(&leaf);
    lease.remove_empty().expect("remove frozen survivor cgroup");
}

fn assert_old_object(path: &std::path::Path, expected_device: u64) {
    let metadata = std::fs::metadata(path).expect("guardian must retain old devpts object");
    assert_eq!(metadata.rdev(), expected_device);
}
