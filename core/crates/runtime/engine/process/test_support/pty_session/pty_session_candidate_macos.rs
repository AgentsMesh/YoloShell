use super::*;
use anyhow::Result;

impl PtySessionProof {
    pub fn path_holders_for_test(&self) -> Result<std::collections::HashSet<u32>> {
        Ok(super::super::macos_path_holders::path_holders(
            &self.tty_path,
            crate::process::constants::MAX_PTY_PROOF_CANDIDATES,
        )?)
    }

    pub fn matches_process_for_test(&self, pid: u32) -> Result<Option<bool>> {
        let mut budget = CapabilityScanBudget::start(SINGLE_TEST_CANDIDATE)?;
        match self.matches_available_process(pid, &mut budget) {
            Ok(matches) => Ok(Some(matches)),
            Err(CapabilityProofError::DescriptorScanDeferred(_)) => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    pub fn matches_exact_writer_for_test(&self, pid: u32) -> Result<bool> {
        let mut budget = CapabilityScanBudget::start(SINGLE_TEST_CANDIDATE)?;
        Ok(self.observes_transferable_process(pid, &mut budget)?)
    }
}

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/pty/session/candidate/pty_session_candidate_test_api_macos.rs"]
mod tests;
