use super::{
    CgroupDirectoryIdentity, LinuxContainmentError, LinuxContainmentRecord, custody_identity,
    custody_validation,
};
use crate::process::ProcessIdentity;
use crate::sha256_digest::Sha256Digest;
use std::fs::File;
use std::os::fd::{AsFd, BorrowedFd, OwnedFd};
use uuid::Uuid;

pub(crate) struct LinuxContainmentCustody {
    root: File,
    leaf: File,
    events: File,
    record: LinuxContainmentRecord,
    bound_root: ProcessIdentity,
    root_identity: CgroupDirectoryIdentity,
    leaf_identity: CgroupDirectoryIdentity,
    recovery_identity_sha256: Sha256Digest,
}

impl LinuxContainmentCustody {
    pub(crate) fn from_transferred(
        root: OwnedFd,
        leaf: OwnedFd,
        record: LinuxContainmentRecord,
        terminal_id: Uuid,
        runtime_epoch: Uuid,
        bound_root: ProcessIdentity,
    ) -> Result<Self, LinuxContainmentError> {
        let root = File::from(root);
        let leaf = File::from(leaf);
        let (root_identity, leaf_identity) = custody_validation::validate(
            &root,
            &leaf,
            &record,
            terminal_id,
            runtime_epoch,
            bound_root,
        )?;
        let recovery_identity_sha256 =
            custody_identity::capture(&record, root_identity, leaf_identity)?;
        let events =
            super::io::open_control(&leaf, super::constants::CGROUP_EVENTS_FILE, libc::O_RDONLY)?;
        Ok(Self {
            root,
            leaf,
            events,
            record,
            bound_root,
            root_identity,
            leaf_identity,
            recovery_identity_sha256,
        })
    }

    pub(crate) fn record(&self) -> &LinuxContainmentRecord {
        &self.record
    }

    pub(crate) fn bound_root(&self) -> ProcessIdentity {
        self.bound_root
    }

    pub(crate) fn identities(&self) -> (CgroupDirectoryIdentity, CgroupDirectoryIdentity) {
        (self.root_identity, self.leaf_identity)
    }

    pub(crate) fn root_descriptor(&self) -> BorrowedFd<'_> {
        self.root.as_fd()
    }

    pub(crate) fn leaf_descriptor(&self) -> BorrowedFd<'_> {
        self.leaf.as_fd()
    }

    pub(crate) fn as_descriptors(&self) -> [BorrowedFd<'_>; super::transfer::DESCRIPTOR_COUNT] {
        [self.root_descriptor(), self.leaf_descriptor()]
    }

    pub(crate) fn into_parts(self) -> (OwnedFd, OwnedFd) {
        (self.root.into(), self.leaf.into())
    }

    pub(crate) fn recovery_identity_sha256(&self) -> Sha256Digest {
        self.recovery_identity_sha256
    }

    pub(crate) fn is_empty(&self) -> Result<bool, LinuxContainmentError> {
        match super::membership::read_member_pids(&self.leaf) {
            Ok(members) => Ok(members.is_empty()),
            Err(_) if self.retained_leaf_was_removed()? => Ok(true),
            Err(error) => Err(error),
        }
    }

    fn retained_leaf_was_removed(&self) -> Result<bool, LinuxContainmentError> {
        super::io::retained_control_was_removed(&self.events)
    }

    pub(crate) fn request_force_termination(&self) -> Result<(), LinuxContainmentError> {
        match super::io::write_control(
            &self.leaf,
            super::constants::CGROUP_KILL_FILE,
            super::constants::KILL_VALUE,
        ) {
            Err(_) if self.retained_leaf_was_removed()? => Ok(()),
            result => result,
        }
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/linux/linux_containment_custody_tests.rs"]
mod tests;
