use super::super::{LinuxContainmentCustody, LinuxContainmentLease};
use super::custody_fixture::prepare_bound_lease;
use super::devpts_fixture::PtyPair;
use super::support::{SleepingChild, leaf_path, wait_empty};
use std::path::PathBuf;

pub(super) struct Fixture {
    pub custody: LinuxContainmentCustody,
    pub path: PathBuf,
    lease: Option<LinuxContainmentLease>,
    child: SleepingChild,
    _pty: PtyPair,
}

impl Fixture {
    pub(super) fn new() -> Self {
        let (lease, child, pty) = prepare_bound_lease();
        let record = lease.record().clone();
        let path = leaf_path(&record);
        let exported = lease.export_secondary_custody().unwrap();
        let (root, leaf) = exported.into_parts();
        let custody = LinuxContainmentCustody::from_transferred(
            root,
            leaf,
            record.clone(),
            record.terminal_id,
            record.runtime_epoch,
            child.identity(),
        )
        .unwrap();
        Self {
            custody,
            path,
            lease: Some(lease),
            child,
            _pty: pty,
        }
    }

    pub(super) fn retire(&mut self) {
        self.child.terminate();
        wait_empty(&self.path);
        self.lease.take().unwrap().remove_empty().unwrap();
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.child.terminate();
        if let Some(lease) = self.lease.take() {
            let _ = lease.remove_empty();
        }
    }
}
