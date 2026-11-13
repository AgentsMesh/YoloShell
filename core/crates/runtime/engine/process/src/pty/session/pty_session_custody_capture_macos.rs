use super::{CapabilityScope, PtySessionProof};
use anyhow::{Result, ensure};
use std::sync::Arc;

use crate::process::pty_custody_recovery_authority_macos::RecoveryBootstrapFence;

const CUSTODY_CAPABILITY: &str = "guardian-custody-pty-object";
const CUSTODY_OBJECT_MISMATCH: &str = "Guardian custody PTY object identity changed";

impl PtySessionProof {
    pub(in crate::process) fn capture_custody(
        custody_probe: crate::process::PtyMasterFdCustodyProbe,
        process_reader: Arc<crate::process::MacosProcessCapabilityManagerReader>,
    ) -> Result<Self> {
        custody_probe.confirm_exact()?;
        let mut proof = Self::capture(
            custody_probe.pty_path().to_path_buf(),
            CUSTODY_CAPABILITY.to_owned(),
        )?;
        ensure!(
            proof.tty_object_identity == Some(custody_probe.pty_object_identity()),
            CUSTODY_OBJECT_MISMATCH
        );
        proof.scope = CapabilityScope::CustodyPtyObject;
        proof.master_alias_state = super::PtyMasterAliasState::NoLocalAlias;
        proof.custody_probe = Some(custody_probe);
        proof.descriptor_capability =
            Some(super::macos_descriptor_authority::MacosDescriptorAuthority::new(process_reader));
        Ok(proof)
    }

    pub(in crate::process) fn capture_recovered_custody(
        custody_probe: crate::process::PtyMasterFdCustodyProbe,
        process_reader: Arc<crate::process::MacosProcessCapabilityManagerReader>,
        recovery_fence: RecoveryBootstrapFence,
    ) -> Result<Self> {
        let _consumed_recovery_fence = recovery_fence;
        let mut proof = Self::capture_custody(custody_probe, process_reader)?;
        let bootstrap_exec = super::bootstrap_exec::BootstrapExecState::armed();
        bootstrap_exec.take_authority()?.confirm_exec()?;
        proof.bootstrap = super::bootstrap_fence::BootstrapFenceState::recovered();
        proof.bootstrap_exec = bootstrap_exec;
        Ok(proof)
    }

    #[cfg(test)]
    pub(crate) fn capture_recovered_custody_for_test(
        custody_probe: crate::process::PtyMasterFdCustodyProbe,
        process_reader: Arc<crate::process::MacosProcessCapabilityManagerReader>,
    ) -> Result<Self> {
        Self::capture_recovered_custody(
            custody_probe,
            process_reader,
            RecoveryBootstrapFence::for_test(),
        )
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/pty/session/custody/pty_session_custody_capture_macos_tests.rs"]
mod tests;
