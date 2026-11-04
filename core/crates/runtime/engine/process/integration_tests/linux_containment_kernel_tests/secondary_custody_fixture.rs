use super::super::{LinuxContainmentLease, LinuxContainmentRecord};
use super::custody_fixture::prepare_bound_lease;
use super::devpts_fixture::PtyPair;
use super::support::{SleepingChild, leaf_path, wait_empty};
use crate::process::{
    LINUX_SECONDARY_CUSTODY_DESCRIPTOR_COUNT, LinuxSecondaryCustody,
    LinuxSecondaryCustodyDescriptorParts, LinuxSecondaryCustodyExport, ProcessIdentity,
};
use crate::sha256_digest::Sha256Digest;
use anyhow::Result;
use std::os::fd::{AsRawFd, OwnedFd};

pub(super) struct SecondaryCustodyFixture {
    lease: Option<LinuxContainmentLease>,
    child: SleepingChild,
    pty: PtyPair,
    record: LinuxContainmentRecord,
    root: ProcessIdentity,
}

impl SecondaryCustodyFixture {
    pub(super) fn new() -> Self {
        let (lease, child, pty) = prepare_bound_lease();
        let record = lease.record().clone();
        let root = child.identity();
        Self {
            lease: Some(lease),
            child,
            pty,
            record,
            root,
        }
    }

    pub(super) fn descriptors(
        &self,
    ) -> (
        [OwnedFd; LINUX_SECONDARY_CUSTODY_DESCRIPTOR_COUNT],
        Sha256Digest,
    ) {
        let containment = self
            .lease
            .as_ref()
            .unwrap()
            .export_secondary_custody()
            .unwrap();
        let exported =
            LinuxSecondaryCustodyExport::capture(self.pty.master.as_raw_fd(), containment).unwrap();
        let digest = exported.recovery_identity_sha256();
        let descriptors = exported
            .as_descriptors()
            .map(|descriptor| descriptor.try_clone_to_owned().unwrap());
        (descriptors, digest)
    }

    pub(super) fn adopt(
        &self,
        descriptors: [OwnedFd; LINUX_SECONDARY_CUSTODY_DESCRIPTOR_COUNT],
        digest: Sha256Digest,
    ) -> Result<LinuxSecondaryCustody> {
        LinuxSecondaryCustody::from_transferred(
            LinuxSecondaryCustodyDescriptorParts::from_ordered(descriptors),
            self.record.clone(),
            self.record.terminal_id,
            self.record.runtime_epoch,
            self.root,
            digest,
            crate::pty_custody::CustodyProfile::ActiveRunning,
        )
    }

    pub(super) fn finish(mut self) {
        self.child.terminate();
        let lease = self.lease.take().unwrap();
        wait_empty(&leaf_path(lease.record()));
        lease.remove_empty().unwrap();
    }
}

impl Drop for SecondaryCustodyFixture {
    fn drop(&mut self) {
        self.child.terminate();
        if let Some(lease) = self.lease.take() {
            let _ = lease.remove_empty();
        }
    }
}
