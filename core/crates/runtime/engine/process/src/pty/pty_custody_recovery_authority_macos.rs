use super::{
    MacosProcessCapabilityIdentity, MacosProcessCapabilityManagerReader, ProcessIdentity,
    PtyMasterFdCustodyProbe, PtySessionProof,
};
use crate::model::RuntimeRecord;
use crate::pty_custody::{CustodyDescriptorSet, CustodyRuntimeIdentity};
use crate::sha256_digest::Sha256Digest;
use anyhow::{Result, ensure};
use std::os::fd::OwnedFd;
use std::sync::Arc;

const DESCRIPTOR_SET_DETAIL: &str = "Guardian recovery process capability is invalid";
const RECOVERY_DIGEST_DETAIL: &str = "Guardian recovery identity digest changed";
const RECOVERY_ROOT_DETAIL: &str = "Guardian recovery runtime child identity changed";
const RECOVERY_BOOTSTRAP_DETAIL: &str = "Guardian recovery runtime bootstrap is not durably ready";

#[doc(hidden)]
pub(in crate::process) struct RecoveryBootstrapFence {
    _private: (),
}

impl RecoveryBootstrapFence {
    fn issue() -> Self {
        Self { _private: () }
    }

    #[cfg(test)]
    pub(crate) fn for_test() -> Self {
        Self::issue()
    }
}

#[doc(hidden)]
pub struct PtyCustodyRecoveryAuthority {
    probe: PtyMasterFdCustodyProbe,
    process_reader: Arc<MacosProcessCapabilityManagerReader>,
    root: ProcessIdentity,
    bootstrap_fence: RecoveryBootstrapFence,
}

impl PtyCustodyRecoveryAuthority {
    #[doc(hidden)]
    pub fn from_transferred(
        probe: PtyMasterFdCustodyProbe,
        descriptor: OwnedFd,
        runtime: &RuntimeRecord,
        descriptor_set: CustodyDescriptorSet,
        expected_digest: Sha256Digest,
    ) -> Result<Self> {
        probe.confirm_exact()?;
        ensure!(
            runtime.bootstrap_delivery.is_ready(),
            RECOVERY_BOOTSTRAP_DETAIL
        );
        let custody_runtime = CustodyRuntimeIdentity::from_record(runtime);
        let expected = process_identity(descriptor_set)?;
        let process_reader =
            MacosProcessCapabilityManagerReader::from_transferred_attested(descriptor, expected)?;
        let observed = super::macos_secondary_custody::MacosSecondaryCustodyIdentity::from_observed(
            &custody_runtime,
            probe.pty_path().to_owned(),
            probe.pty_object_identity(),
            process_reader.identity(),
        );
        ensure!(
            observed.digest()? == expected_digest,
            RECOVERY_DIGEST_DETAIL
        );
        Ok(Self {
            probe,
            process_reader,
            root: ProcessIdentity {
                pid: custody_runtime.child_pid,
                start_time: custody_runtime.child_start_time,
            },
            bootstrap_fence: RecoveryBootstrapFence::issue(),
        })
    }

    pub(in crate::process) fn into_proof(self, root: ProcessIdentity) -> Result<PtySessionProof> {
        ensure!(root == self.root, RECOVERY_ROOT_DETAIL);
        PtySessionProof::capture_recovered_custody(
            self.probe,
            self.process_reader,
            self.bootstrap_fence,
        )
    }
}

fn process_identity(set: CustodyDescriptorSet) -> Result<MacosProcessCapabilityIdentity> {
    match set {
        CustodyDescriptorSet::MacosProcessCapability {
            reader_handle,
            writer_handle,
        } => Ok(MacosProcessCapabilityIdentity::from_attestation(
            reader_handle,
            writer_handle,
        )),
        _ => anyhow::bail!(DESCRIPTOR_SET_DETAIL),
    }
}
