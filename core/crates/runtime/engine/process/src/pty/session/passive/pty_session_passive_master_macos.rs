use super::PtySessionProof;
use super::macos_object_identity::MacosPtyObjectIdentity;
use anyhow::{Context, Result};
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd};
use std::path::{Path, PathBuf};

#[path = "pty_session_passive_master_drain_macos.rs"]
mod drain;
#[cfg(any(
    all(test, feature = "yolo-integration-tests"),
    feature = "adversarial-test-hooks"
))]
pub use drain::install_pending_once;
#[path = "pty_session_passive_master_clone_macos.rs"]
mod clone;
#[path = "pty_session_passive_master_holder_lineage_macos.rs"]
mod holder_lineage;
#[path = "pty_session_passive_master_holders_macos.rs"]
mod holders;
pub(in crate::process) use holders::MacosPtyHolderExclusions;

const MISSING_OBJECT_LEASE_DETAIL: &str = "bound PTY proof has no exact master object lease";
const MISSING_PROCESS_READER_DETAIL: &str = "bound PTY proof has no process capability reader";
const TRANSFERRED_FLAGS_CONTEXT: &str = "configure transferred passive PTY master descriptor";
const TRANSFERRED_OBJECT_CHANGED_DETAIL: &str = "passive PTY master object identity changed";

pub(crate) struct MacosPassivePtyMaster {
    descriptor: OwnedFd,
    path: PathBuf,
    identity: MacosPtyObjectIdentity,
}

impl MacosPassivePtyMaster {
    pub(crate) fn duplicate_exact(proof: &PtySessionProof) -> Result<Self> {
        let lease = proof
            .pty_object_lease
            .as_ref()
            .context(MISSING_OBJECT_LEASE_DETAIL)?;
        let descriptor = lease.duplicate_exact(proof)?;
        Self::from_transferred_exact(descriptor, proof)
    }

    pub(crate) fn from_transferred(descriptor: OwnedFd) -> Result<Self> {
        configure_transferred(&descriptor)?;
        let (path, identity) = super::macos_object_lease::inspect_descriptor(&descriptor)?;
        Ok(Self {
            descriptor,
            path,
            identity,
        })
    }

    pub(crate) fn from_transferred_exact(
        descriptor: OwnedFd,
        proof: &PtySessionProof,
    ) -> Result<Self> {
        configure_transferred(&descriptor)?;
        let (path, identity) = super::macos_object_lease::validate_descriptor(&descriptor, proof)?;
        Ok(Self {
            descriptor,
            path,
            identity,
        })
    }

    pub(crate) fn descriptor(&self) -> BorrowedFd<'_> {
        self.descriptor.as_fd()
    }

    pub(crate) fn matches(&self, proof: &PtySessionProof) -> bool {
        self.path == proof.tty_path && proof.tty_object_identity == Some(self.identity)
    }

    pub(super) fn revalidate_exact(&self, proof: &PtySessionProof) -> Result<()> {
        let (path, identity) =
            super::macos_object_lease::validate_descriptor(&self.descriptor, proof)?;
        anyhow::ensure!(
            path == self.path && identity == self.identity,
            TRANSFERRED_OBJECT_CHANGED_DETAIL
        );
        Ok(())
    }

    pub(crate) fn slave_path(&self) -> &Path {
        &self.path
    }

    pub(in crate::process) fn object_identity(&self) -> MacosPtyObjectIdentity {
        self.identity
    }

    pub(in crate::process) fn into_descriptor(self) -> OwnedFd {
        self.descriptor
    }
}

fn configure_transferred(descriptor: &OwnedFd) -> Result<()> {
    super::super::macos_process_capability_fd::set_close_on_exec(descriptor.as_raw_fd(), true)
        .context(TRANSFERRED_FLAGS_CONTEXT)?;
    drain::configure_nonblocking(descriptor).context(TRANSFERRED_FLAGS_CONTEXT)
}

impl PtySessionProof {
    pub(in crate::process) fn secondary_process_reader(
        &self,
    ) -> Result<&super::super::MacosProcessCapabilityManagerReader> {
        self.descriptor_capability
            .as_ref()
            .map(super::macos_descriptor_authority::MacosDescriptorAuthority::process_reader)
            .context(MISSING_PROCESS_READER_DETAIL)
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../../integration_tests/pty/session/passive/pty_session_passive_master_reuse_macos_tests.rs"]
mod reuse_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../../integration_tests/pty/session/passive/pty_session_passive_master_macos_tests.rs"]
mod tests;
