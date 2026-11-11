use super::linux_containment::{
    LinuxContainmentCustody, LinuxContainmentDescriptorExport, LinuxContainmentRecord,
};
use super::{LinuxPassivePtyMaster, LinuxPtySlaveIdentity, ProcessIdentity};
use crate::pty_custody::{CustodyDescriptorSet, CustodyProfile};
use crate::sha256_digest::Sha256Digest;
use anyhow::{Result, ensure};
use serde::Serialize;
use std::os::fd::{BorrowedFd, OwnedFd, RawFd};
use uuid::Uuid;

pub const LINUX_SECONDARY_CUSTODY_DESCRIPTOR_COUNT: usize = 3;
const IDENTITY_DIGEST_DOMAIN: &str = "yolo/linux-secondary-custody-identity/v1";
const IDENTITY_DIGEST_MISMATCH_DETAIL: &str =
    "transferred Linux custody descriptors have another recovery identity";
const MISSING_PTY_DEVICE_DETAIL: &str = "Linux containment has no bound PTY device";
const PROFILE_DETAIL: &str = "Linux custody import requires an active custody profile";

pub struct LinuxSecondaryCustodyDescriptorParts {
    pty_master: OwnedFd,
    cgroup_root: OwnedFd,
    cgroup_leaf: OwnedFd,
}

pub struct LinuxSecondaryCustodyExport {
    pty_master: LinuxPassivePtyMaster,
    containment: LinuxContainmentDescriptorExport,
    recovery_identity_sha256: Sha256Digest,
}

pub struct LinuxSecondaryCustody {
    pty_master: LinuxPassivePtyMaster,
    containment: LinuxContainmentCustody,
    recovery_identity_sha256: Sha256Digest,
}

#[derive(Serialize)]
struct CompositeIdentity {
    domain: &'static str,
    containment: Sha256Digest,
    pty_slave: LinuxPtySlaveIdentity,
}

impl LinuxSecondaryCustodyDescriptorParts {
    pub fn from_ordered(
        [pty_master, cgroup_root, cgroup_leaf]: [OwnedFd; LINUX_SECONDARY_CUSTODY_DESCRIPTOR_COUNT],
    ) -> Self {
        Self {
            pty_master,
            cgroup_root,
            cgroup_leaf,
        }
    }
}

impl LinuxSecondaryCustodyExport {
    pub(crate) fn capture(
        master: RawFd,
        containment: LinuxContainmentDescriptorExport,
    ) -> Result<Self> {
        let pty_master = LinuxPassivePtyMaster::duplicate_exact(master, containment.pty_device())?;
        let recovery_identity_sha256 = composite_digest(
            containment.recovery_identity_sha256(),
            pty_master.slave_identity(),
        )?;
        Ok(Self {
            pty_master,
            containment,
            recovery_identity_sha256,
        })
    }

    pub fn as_descriptors(
        &self,
    ) -> [BorrowedFd<'_>; LINUX_SECONDARY_CUSTODY_DESCRIPTOR_COUNT] {
        [
            self.pty_master.descriptor(),
            self.containment.root_descriptor(),
            self.containment.leaf_descriptor(),
        ]
    }

    pub fn recovery_identity_sha256(&self) -> Sha256Digest {
        self.recovery_identity_sha256
    }

    pub const fn descriptor_set(&self) -> CustodyDescriptorSet {
        CustodyDescriptorSet::LinuxContainment
    }
}

impl LinuxSecondaryCustody {
    pub fn from_transferred(
        parts: LinuxSecondaryCustodyDescriptorParts,
        record: LinuxContainmentRecord,
        terminal_id: Uuid,
        runtime_epoch: Uuid,
        child: ProcessIdentity,
        expected_digest: Sha256Digest,
        profile: CustodyProfile,
    ) -> Result<Self> {
        ensure!(profile.is_active(), PROFILE_DETAIL);
        let expected_device = record
            .pty_device
            .filter(|device| *device != u64::default())
            .ok_or_else(|| anyhow::anyhow!(MISSING_PTY_DEVICE_DETAIL))?;
        let pty_master =
            LinuxPassivePtyMaster::from_transferred(parts.pty_master, expected_device)?;
        let containment = LinuxContainmentCustody::from_transferred(
            parts.cgroup_root,
            parts.cgroup_leaf,
            record,
            terminal_id,
            runtime_epoch,
            child,
        )?;
        let recovery_identity_sha256 = composite_digest(
            containment.recovery_identity_sha256(),
            pty_master.slave_identity(),
        )?;
        ensure!(
            recovery_identity_sha256 == expected_digest,
            IDENTITY_DIGEST_MISMATCH_DETAIL
        );
        Ok(Self {
            pty_master,
            containment,
            recovery_identity_sha256,
        })
    }

    pub fn recovery_identity_sha256(&self) -> Sha256Digest {
        self.recovery_identity_sha256
    }

    pub fn closure_settled(&self) -> Result<bool> {
        self.containment.is_empty().map_err(anyhow::Error::new)
    }

    pub fn request_active_settlement(&self) -> Result<bool> {
        if self.closure_settled()? {
            return Ok(true);
        }
        self.containment
            .request_force_termination()
            .map_err(anyhow::Error::new)?;
        Ok(false)
    }
}

fn composite_digest(
    containment: Sha256Digest,
    pty_slave: LinuxPtySlaveIdentity,
) -> Result<Sha256Digest> {
    Sha256Digest::capture(&CompositeIdentity {
        domain: IDENTITY_DIGEST_DOMAIN,
        containment,
        pty_slave,
    })
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/linux/linux_secondary_custody_tests.rs"]
mod tests;
