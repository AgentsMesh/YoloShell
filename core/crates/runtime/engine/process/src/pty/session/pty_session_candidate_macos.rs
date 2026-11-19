use super::PtySessionProof;
#[cfg(test)]
use super::macos::EnvironmentCapability;
use super::macos::environment_capability;
use super::macos_contract::{CapabilityProofError, CapabilityProofResult, CapabilityScanBudget};
use super::macos_descriptor_capability::has_stable_descriptor_capability;
use super::macos_descriptors::{has_stable_tty_descriptor, observes_tty_descriptor};
use crate::process::sigchld_waitability::{DescriptorScanGuard, try_descriptor_scan_guard};
#[path = "pty_session_candidate_fallback_macos.rs"]
mod fallback;
#[path = "pty_session_candidate_fence_macos.rs"]
mod fence;
#[path = "pty_session_root_exclusion_macos.rs"]
mod root_exclusion;
pub(super) use fallback::match_tty_after_environment;
#[cfg(test)]
use fallback::{CAPABILITY_QUERY_FAILURE_CONTEXT, DESCRIPTOR_QUERY_FAILURE_CONTEXT};
#[cfg(test)]
pub(super) use fallback::{match_environment_if_tty, match_tty_if_capability};
#[cfg(test)]
pub(super) use fence::CANDIDATE_GENERATION_FENCE_CONTEXT;
pub(super) use fence::{CandidateEvaluation, fenced_candidate};
pub(super) use root_exclusion::exclude_root;

#[cfg(any(test, feature = "runtime-component-tests"))]
pub(super) const SINGLE_TEST_CANDIDATE: usize = 1;

impl PtySessionProof {
    pub(super) fn matches_available_process(
        &self,
        pid: u32,
        budget: &mut CapabilityScanBudget,
    ) -> CapabilityProofResult<bool> {
        let _scan_guard = self.descriptor_scan_guard()?;
        let has_tty = has_stable_tty_descriptor(pid, self.pty_object_identity()?, budget)?;
        if !has_tty {
            return Ok(false);
        }
        self.matches_owned_process_guarded(pid, budget)
    }

    pub(super) fn matches_released_process(
        &self,
        pid: u32,
        budget: &mut CapabilityScanBudget,
    ) -> CapabilityProofResult<bool> {
        if self.descriptor_capability.is_some() {
            let _scan_guard = self.descriptor_scan_guard()?;
            return self.matches_owned_process_guarded(pid, budget);
        }
        let environment =
            environment_capability(pid, self.environment_name(), &self.capability, budget);
        match_tty_after_environment(environment, || {
            has_stable_tty_descriptor(pid, self.pty_object_identity()?, budget)
        })
    }

    pub(super) fn matches_available_after_writer_release(
        &self,
        pid: u32,
        budget: &mut CapabilityScanBudget,
    ) -> CapabilityProofResult<bool> {
        self.matches_released_tty_process(pid, budget)
    }

    pub(super) fn matches_escaped_process(
        &self,
        pid: u32,
        budget: &mut CapabilityScanBudget,
    ) -> CapabilityProofResult<bool> {
        let _scan_guard = self.descriptor_scan_guard()?;
        self.matches_owned_process_guarded(pid, budget)
    }

    pub(super) fn matches_released_tty_process(
        &self,
        pid: u32,
        budget: &mut CapabilityScanBudget,
    ) -> CapabilityProofResult<bool> {
        let _scan_guard = self.descriptor_scan_guard()?;
        has_stable_tty_descriptor(pid, self.pty_object_identity()?, budget)
    }

    pub(super) fn observes_released_tty_process(
        &self,
        pid: u32,
        budget: &mut CapabilityScanBudget,
    ) -> CapabilityProofResult<bool> {
        let _scan_guard = self.descriptor_scan_guard()?;
        observes_tty_descriptor(pid, self.pty_object_identity()?, budget)
    }

    pub(super) fn descriptor_scan_guard(
        &self,
    ) -> CapabilityProofResult<Option<DescriptorScanGuard>> {
        self.descriptor_capability
            .as_ref()
            .map(|_| {
                try_descriptor_scan_guard()
                    .ok_or_else(CapabilityProofError::descriptor_scan_deferred)
            })
            .transpose()
    }

    fn matches_owned_process_guarded(
        &self,
        pid: u32,
        budget: &mut CapabilityScanBudget,
    ) -> CapabilityProofResult<bool> {
        let environment =
            environment_capability(pid, self.environment_name(), &self.capability, budget);
        let Some(descriptor) = self.descriptor_capability.as_ref() else {
            return environment.map(|capability| capability.matches);
        };
        match environment {
            Ok(capability) if capability.matches => Ok(true),
            Ok(_) | Err(CapabilityProofError::CandidateLocalQuery(_)) => {
                has_stable_descriptor_capability(pid, descriptor.writer_handle(), budget)
            }
            Err(error) => Err(error),
        }
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/pty/session/descriptor/pty_session_descriptor_capability_process_tests.rs"]
mod descriptor_process_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/pty/session/candidate/pty_session_candidate_fallback_tests.rs"]
mod fallback_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/pty/session/candidate/pty_session_candidate_matrix_tests.rs"]
mod matrix_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/pty/session/candidate/pty_session_candidate_process_tests.rs"]
mod process_tests;
#[cfg(any(all(test, feature = "yolo-integration-tests"), feature = "runtime-component-tests"))]
#[path = "../../../test_support/pty_session/pty_session_candidate_macos.rs"]
#[cfg(any(test, feature = "adversarial-test-hooks"))]
mod test_api;
