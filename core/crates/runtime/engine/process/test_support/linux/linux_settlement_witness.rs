use super::super::{
    LinuxContainmentRecord, LinuxContainmentRecovery, LinuxContainmentRecoveryPoll,
    LinuxContainmentSettlement,
};
use super::custody_fixture::prepare_bound_lease;
use super::record::DurableRecord;
use super::support::{KERNEL_POLL_INTERVAL, KERNEL_SETTLE_TIMEOUT, leaf_path, wait_empty};
use crate::process::ProcessIdentity;
use std::time::Instant;

pub(super) fn settle() -> (LinuxContainmentSettlement, DurableRecord) {
    let (lease, mut child, _pty) = prepare_bound_lease();
    let record = lease.record().clone();
    let durable = DurableRecord::write(&record);
    let root = child.identity();
    child.terminate();
    wait_empty(&leaf_path(&record));
    drop(lease);
    let mut recovery = LinuxContainmentRecovery::open(
        record.clone(),
        record.terminal_id,
        record.runtime_epoch,
        root,
    )
    .expect("open complete recovery");
    let deadline = Instant::now() + KERNEL_SETTLE_TIMEOUT;
    loop {
        match recovery.poll().expect("poll complete recovery") {
            LinuxContainmentRecoveryPoll::Settled { leaf_removed: true } => break,
            LinuxContainmentRecoveryPoll::Settled {
                leaf_removed: false,
            } => {
                panic!("a retained leaf cannot authorize settlement");
            }
            LinuxContainmentRecoveryPoll::Pending => {}
        }
        assert!(Instant::now() < deadline, "recovery did not settle");
        std::thread::sleep(KERNEL_POLL_INTERVAL);
    }
    (
        recovery.into_settlement().expect("retain settlement proof"),
        durable,
    )
}

pub(super) fn observe(
    record: &LinuxContainmentRecord,
) -> Result<Option<LinuxContainmentSettlement>, super::super::LinuxContainmentError> {
    LinuxContainmentSettlement::observe_settled(
        record.clone(),
        record.terminal_id,
        record.runtime_epoch,
        ProcessIdentity {
            pid: record.bound_pid.expect("complete bound pid"),
            start_time: record.bound_start_time.expect("complete bound start time"),
        },
    )
}
