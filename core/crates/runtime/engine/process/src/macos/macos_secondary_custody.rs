use super::pty_session_proof::MacosPtyObjectIdentity;
use super::{
    MacosPassivePtyMaster, MacosProcessCapabilityIdentity, MacosProcessCapabilityManagerReader,
    ProcessIdentity, PtySessionProof,
};
use crate::pty_custody::{CustodyDescriptorSet, CustodyProfile, CustodyRuntimeIdentity};
use crate::sha256_digest::Sha256Digest;
use anyhow::{Result, ensure};
use serde::Serialize;
use std::os::fd::{AsFd, BorrowedFd, OwnedFd};
use std::path::PathBuf;
use std::sync::Arc;
pub const MACOS_SECONDARY_CUSTODY_DESCRIPTOR_COUNT: usize = 2;
const IDENTITY_DIGEST_MISMATCH_DETAIL: &str =
    "transferred macOS custody descriptors have another recovery identity";
const DESCRIPTOR_SET_DETAIL: &str = "macOS custody descriptor identity attestation is invalid";
const PROFILE_DETAIL: &str = "macOS custody import requires an active custody profile";
const RUNTIME_IDENTITY_DETAIL: &str = "macOS custody runtime identity is incomplete";
#[path = "macos_secondary_custody_identity.rs"]
mod identity;
#[path = "macos_secondary_custody_import.rs"]
mod import;
#[path = "macos_secondary_custody_namespace_loss.rs"]
mod namespace_loss;
#[cfg(any(test, feature = "adversarial-test-hooks"))]
#[path = "macos_secondary_custody_recovery.rs"]
mod recovery;
#[path = "macos_secondary_custody_settlement.rs"]
mod settlement;
use import::arm_transferred_liveness;
pub use namespace_loss::{
    MacosNamespaceLossContainmentAuthority, MacosNamespaceLossFinalized,
    MacosNamespaceLossSealFinalization, MacosPtyKernelEof,
};
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct MacosSecondaryCustodyIdentity {
    runtime: CustodyRuntimeIdentity,
    pty_slave_path: PathBuf,
    pty_object: MacosPtyObjectIdentity,
    process_capability: MacosProcessCapabilityIdentity,
}

pub struct MacosSecondaryCustodyDescriptorParts {
    pty_master: OwnedFd,
    process_reader: OwnedFd,
}

pub struct MacosSecondaryCustodyExport {
    pty_master: MacosPassivePtyMaster,
    process_reader: OwnedFd,
    identity: MacosSecondaryCustodyIdentity,
    digest: Sha256Digest,
}

pub struct MacosSecondaryCustody {
    pty_master: MacosPassivePtyMaster,
    namespace_loss_seal: super::pty_session_proof::MacosNamespaceLossReopenSeal,
    process_reader: Arc<MacosProcessCapabilityManagerReader>,
    identity: MacosSecondaryCustodyIdentity,
    digest: Sha256Digest,
}

impl MacosSecondaryCustodyDescriptorParts {
    pub fn from_ordered(
        [pty_master, process_reader]: [OwnedFd; MACOS_SECONDARY_CUSTODY_DESCRIPTOR_COUNT],
    ) -> Self {
        Self {
            pty_master,
            process_reader,
        }
    }
}

impl MacosSecondaryCustodyExport {
    pub(crate) fn capture(
        proof: &PtySessionProof,
        runtime: &CustodyRuntimeIdentity,
    ) -> Result<Self> {
        ensure!(runtime.is_complete(), RUNTIME_IDENTITY_DETAIL);
        let reader = proof.secondary_process_reader()?;
        let pty_master = MacosPassivePtyMaster::duplicate_exact(proof)?;
        let process_reader = reader.duplicate_for_secondary()?;
        let identity =
            MacosSecondaryCustodyIdentity::capture(runtime, &pty_master, reader.identity());
        let digest = identity.digest()?;
        proof.bind_secondary_custody_recovery(runtime, digest)?;
        Ok(Self {
            pty_master,
            process_reader,
            identity,
            digest,
        })
    }

    pub fn as_descriptors(&self) -> [BorrowedFd<'_>; MACOS_SECONDARY_CUSTODY_DESCRIPTOR_COUNT] {
        [self.pty_master.descriptor(), self.process_reader.as_fd()]
    }

    pub(crate) fn into_parts(self) -> MacosSecondaryCustodyDescriptorParts {
        MacosSecondaryCustodyDescriptorParts {
            pty_master: self.pty_master.into_descriptor(),
            process_reader: self.process_reader,
        }
    }

    pub(crate) fn identity(&self) -> &MacosSecondaryCustodyIdentity {
        &self.identity
    }

    pub fn recovery_identity_sha256(&self) -> Sha256Digest {
        self.digest
    }

    pub fn descriptor_set(&self) -> CustodyDescriptorSet {
        CustodyDescriptorSet::macos_process_capability(
            self.identity.process_capability.reader_handle(),
            self.identity.process_capability.writer_handle(),
        )
    }
}

impl MacosSecondaryCustody {
    pub fn from_transferred(
        parts: MacosSecondaryCustodyDescriptorParts,
        runtime: &CustodyRuntimeIdentity,
        expected_digest: Sha256Digest,
        descriptor_set: CustodyDescriptorSet,
        profile: CustodyProfile,
    ) -> Result<Self> {
        ensure!(runtime.is_complete(), RUNTIME_IDENTITY_DETAIL);
        ensure!(profile.is_active(), PROFILE_DETAIL);
        ensure!(descriptor_set.is_well_formed(), DESCRIPTOR_SET_DETAIL);
        let expected_process = match descriptor_set {
            CustodyDescriptorSet::MacosProcessCapability {
                reader_handle,
                writer_handle,
            } => MacosProcessCapabilityIdentity::from_attestation(reader_handle, writer_handle),
            _ => anyhow::bail!(DESCRIPTOR_SET_DETAIL),
        };
        let MacosSecondaryCustodyDescriptorParts {
            pty_master,
            process_reader,
        } = parts;
        let pty_master = MacosPassivePtyMaster::from_transferred(pty_master)?;
        let process_reader = MacosProcessCapabilityManagerReader::from_transferred_attested(
            process_reader,
            expected_process,
        )?;
        let identity =
            MacosSecondaryCustodyIdentity::capture(runtime, &pty_master, process_reader.identity());
        let digest = identity.digest()?;
        ensure!(digest == expected_digest, IDENTITY_DIGEST_MISMATCH_DETAIL);
        let root = ProcessIdentity {
            pid: runtime.child_pid,
            start_time: runtime.child_start_time,
        };
        arm_transferred_liveness(&process_reader, root)?;
        Ok(Self {
            pty_master,
            namespace_loss_seal: super::pty_session_proof::MacosNamespaceLossReopenSeal::new(),
            process_reader,
            identity,
            digest,
        })
    }

    pub fn as_descriptors(&self) -> [BorrowedFd<'_>; MACOS_SECONDARY_CUSTODY_DESCRIPTOR_COUNT] {
        [
            self.pty_master.descriptor(),
            self.process_reader.as_descriptor(),
        ]
    }

    pub(crate) fn identity(&self) -> &MacosSecondaryCustodyIdentity {
        &self.identity
    }

    pub fn recovery_identity_sha256(&self) -> Sha256Digest {
        self.digest
    }

    #[doc(hidden)]
    pub fn master_fd_custody_probe(&self) -> Result<crate::process::PtyMasterFdCustodyProbe> {
        crate::process::PtyMasterFdCustodyProbe::capture(&self.pty_master)
    }
}

#[cfg(any(
    all(test, feature = "yolo-test-support"),
    feature = "adversarial-test-hooks"
))]
include!("../../test_support/macos/macos_secondary_custody.rs");
#[cfg(all(test, feature = "yolo-integration-tests"))]
include!("../../integration_tests/macos/macos_secondary_custody_test_wiring.rs");
