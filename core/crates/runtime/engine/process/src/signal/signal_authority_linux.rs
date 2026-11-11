use super::{DestructiveSessionAccess, DestructiveSessionUnavailable};
use crate::process::LiveProcessIdentity;
use crate::process::linux_containment::{
    LinuxContainmentCommitReady, LinuxContainmentDescriptorExport, LinuxContainmentPreparation,
};
use anyhow::Result;
use std::collections::HashSet;

impl DestructiveSessionAccess {
    pub(in crate::process) fn poll_linux_frozen_members(
        &mut self,
        request_freeze: bool,
    ) -> Result<Option<HashSet<LiveProcessIdentity>>> {
        self.containment
            .as_mut()
            .ok_or_else(DestructiveSessionUnavailable::new)?
            .poll_frozen_members(request_freeze)
            .map_err(Into::into)
    }

    pub(in crate::process) fn has_linux_containment(&self) -> bool {
        self.containment.is_some()
    }

    pub(in crate::process) fn linux_containment_was_removed(&self) -> bool {
        self.containment_removed
    }

    pub(in crate::process) fn export_linux_containment_custody(
        &self,
    ) -> Result<LinuxContainmentDescriptorExport> {
        self.containment
            .as_ref()
            .ok_or_else(DestructiveSessionUnavailable::new)?
            .export_secondary_custody()
            .map_err(Into::into)
    }

    pub(in crate::process) fn prepare_linux_containment(
        &mut self,
        expected_survivors: &HashSet<LiveProcessIdentity>,
    ) -> Result<LinuxContainmentPreparation> {
        let preparation = self
            .containment
            .as_mut()
            .ok_or_else(DestructiveSessionUnavailable::new)?
            .prepare_cleanup(expected_survivors)?;
        commit_preparation(
            &mut self.containment,
            &mut self.authority,
            &mut self.containment_removed,
            &preparation,
        );
        Ok(preparation)
    }

    pub(in crate::process) fn validate_linux_retained(
        &mut self,
        ready: &LinuxContainmentCommitReady,
    ) -> Result<bool> {
        self.containment
            .as_mut()
            .ok_or_else(DestructiveSessionUnavailable::new)?
            .validate_retained(ready)
            .map_err(Into::into)
    }

    pub(in crate::process) fn settle_linux_retained_after_master_release(
        &mut self,
        ready: &LinuxContainmentCommitReady,
    ) -> Result<bool> {
        let _retained_still_frozen = self.validate_linux_retained(ready)?;
        Ok(false)
    }
}

fn commit_preparation<T, A>(
    containment: &mut Option<T>,
    authority: &mut Option<A>,
    removed: &mut bool,
    preparation: &LinuxContainmentPreparation,
) {
    if matches!(preparation, LinuxContainmentPreparation::Removed) {
        clear_authority(containment, authority);
        *removed = true;
    }
}

fn clear_authority<T, A>(containment: &mut Option<T>, authority: &mut Option<A>) {
    *containment = None;
    *authority = None;
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/signal/signal_authority_linux_tests.rs"]
mod tests;
