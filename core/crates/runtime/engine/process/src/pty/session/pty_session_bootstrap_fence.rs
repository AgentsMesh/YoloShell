use super::{
    CapabilityCandidates, CapabilityScanAdvance, CapabilityScanResult, CapabilityScanStatus,
    PtySessionProof,
};
use anyhow::Result;
#[path = "pty_session_fresh_bootstrap.rs"]
mod fresh;

#[derive(Clone, Copy, Default)]
pub(super) struct BootstrapFenceState {
    #[cfg(target_os = "linux")]
    pub(in crate::process) linux_origin: Option<crate::process::ProcessIdentity>,
    exec_fenced: bool,
    recovery_fenced: bool,
    capability_covered: bool,
}

impl BootstrapFenceState {
    pub(super) fn recovered() -> Self {
        Self {
            recovery_fenced: true,
            ..Self::default()
        }
    }

    pub(super) fn without_retained_coverage(self) -> Self {
        Self {
            #[cfg(target_os = "linux")]
            linux_origin: self.linux_origin,
            exec_fenced: self.exec_fenced,
            recovery_fenced: self.recovery_fenced,
            capability_covered: false,
        }
    }

    fn exec_is_fenced(self, has_transferable_descriptor: bool) -> bool {
        self.recovery_fenced || (self.exec_fenced && !has_transferable_descriptor)
    }

    fn is_recovery(self) -> bool {
        self.recovery_fenced
    }
}

impl PtySessionProof {
    pub fn capture_bootstrap_bound(
        master: &dyn portable_pty::MasterPty,
        capability: String,
    ) -> Result<Self> {
        let mut proof = Self::capture_bound(master, capability)?;
        proof.bootstrap.exec_fenced = true;
        proof.bootstrap.capability_covered = false;
        proof.bootstrap_exec = super::bootstrap_exec::BootstrapExecState::armed();
        Ok(proof)
    }

    pub fn take_bootstrap_exec_authority(&self) -> Result<super::BootstrapExecTransitionAuthority> {
        self.bootstrap_exec.take_authority()
    }

    pub(in crate::process) fn accept_explicit_bootstrap_exec_failure(
        &self,
        proof: super::ExplicitBootstrapExecFailureProof,
    ) -> Result<()> {
        self.bootstrap_exec.consume_failure(proof)
    }

    #[cfg(target_os = "macos")]
    pub(in crate::process) fn bootstrap_exec_is_fenced(&self) -> bool {
        self.bootstrap
            .exec_is_fenced(self.has_transferable_descriptor())
    }

    #[cfg(target_os = "macos")]
    pub(in crate::process) fn is_recovery_custody(&self) -> bool {
        self.is_custody_pty_object() && self.bootstrap.is_recovery()
    }

    #[cfg(target_os = "macos")]
    pub(in crate::process) fn bootstrap_exec_was_confirmed(&self) -> bool {
        self.bootstrap_exec.is_confirmed()
    }

    #[cfg(target_os = "macos")]
    pub(super) fn bootstrap_capability_scan_result(&self) -> Option<CapabilityScanResult> {
        self.bootstrap_capability_coverage_is_retained()
            .then(|| CapabilityScanResult {
                candidates: CapabilityCandidates {
                    status: CapabilityScanStatus::Pending,
                    ..CapabilityCandidates::default()
                },
                advance: CapabilityScanAdvance::reset(),
                // Frozen discovery owns fork acknowledgement after retained coverage.
                scheduling_status: CapabilityScanStatus::Pending,
                coverage_status: CapabilityScanStatus::Pending,
            })
    }

    #[cfg(target_os = "macos")]
    pub(in crate::process) fn bootstrap_capability_coverage_is_retained(&self) -> bool {
        self.bootstrap_exec_is_fenced() && self.bootstrap.capability_covered
    }

    #[cfg(target_os = "macos")]
    pub(in crate::process) fn retain_bootstrap_capability_coverage(&mut self) {
        self.bootstrap.capability_covered = self.bootstrap_exec_is_fenced();
    }

    #[cfg(target_os = "macos")]
    pub(in crate::process) fn clear_bootstrap_capability_coverage(&mut self) {
        self.bootstrap.capability_covered = false;
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn install_process_capability(
        &mut self,
        authority: std::sync::Arc<super::super::MacosProcessCapabilityManagerReader>,
    ) -> Result<()> {
        anyhow::ensure!(
            authority.writer_handle() != u64::default(),
            super::capture::EMPTY_CAPABILITY_DETAIL
        );
        anyhow::ensure!(
            self.descriptor_capability.is_none(),
            super::capture::EMPTY_CAPABILITY_DETAIL
        );
        self.descriptor_capability =
            Some(super::macos_descriptor_authority::MacosDescriptorAuthority::new(authority));
        self.bootstrap = BootstrapFenceState::default();
        Ok(())
    }
}

#[cfg(all(test, target_os = "macos"))]
#[cfg(feature = "yolo-unit-tests")]
#[path = "../../../unit_tests/pty_session_bootstrap_fence_inline_tests.rs"]
mod tests;
