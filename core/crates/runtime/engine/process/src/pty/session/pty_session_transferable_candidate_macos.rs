use super::PtySessionProof;
use super::macos_contract::{CapabilityProofError, CapabilityProofResult, CapabilityScanBudget};
use super::macos_descriptor_capability::observes_descriptor_capability;
use anyhow::anyhow;

const TRANSFERABLE_DESCRIPTOR_MISSING_DETAIL: &str =
    "transferable process capability has no descriptor authority";

impl PtySessionProof {
    pub(super) fn observes_transferable_process(
        &self,
        pid: u32,
        budget: &mut CapabilityScanBudget,
    ) -> CapabilityProofResult<bool> {
        let _scan_guard = self.descriptor_scan_guard()?;
        let descriptor = self.descriptor_capability.as_ref().ok_or_else(|| {
            CapabilityProofError::invariant(anyhow!(TRANSFERABLE_DESCRIPTOR_MISSING_DETAIL))
        })?;
        observes_descriptor_capability(pid, descriptor.writer_handle(), budget)
    }
}
