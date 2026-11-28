use super::{ForceOutput, PtyForceTermination};
use crate::process::{FrozenProcessTree, ProcessClosureVersion};
use crate::worker::pty_output_progress::{
    KernelEofCommitOutcome, KernelEofValidation, KernelEofValidationRequest, KernelPtyEofAuthority,
};
use anyhow::Result;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum EmptyClosureOutputRequirement {
    None,
    KernelEof,
}

impl EmptyClosureOutputRequirement {
    pub(crate) fn from_frozen_tree(tree: &FrozenProcessTree) -> Self {
        if tree.empty_gone_requires_output_eof() {
            Self::KernelEof
        } else {
            Self::None
        }
    }
}

impl PtyForceTermination {
    pub(crate) fn empty_closure_output_requirement(
        &self,
        tree: &FrozenProcessTree,
    ) -> EmptyClosureOutputRequirement {
        if tree.pty_master_released() {
            return EmptyClosureOutputRequirement::None;
        }
        #[cfg(test)]
        if let Some(requirement) = self.empty_closure_output_requirement {
            return requirement;
        }
        EmptyClosureOutputRequirement::from_frozen_tree(tree)
    }

    #[cfg(test)]
    pub(crate) fn requiring_empty_closure_output_for_test(
        requirement: EmptyClosureOutputRequirement,
    ) -> Self {
        let mut force = Self::new();
        force.empty_closure_output_requirement = Some(requirement);
        force
    }
}

impl ForceOutput<'_> {
    pub(crate) fn acquire_kernel_eof_authority(
        &mut self,
        version: &ProcessClosureVersion,
    ) -> Result<Option<KernelPtyEofAuthority>> {
        match self {
            Self::Reader(output) => output.acquire_kernel_eof_authority(version),
            Self::Exclusive(output) => output.acquire_kernel_eof_authority(version),
            #[cfg(test)]
            Self::Scripted(output) => Ok(output.acquire_kernel_eof_authority(version)),
        }
    }

    pub(crate) fn validates_kernel_eof_authority(
        &mut self,
        authority: &KernelPtyEofAuthority,
        version: &ProcessClosureVersion,
    ) -> Result<bool> {
        match self {
            Self::Reader(output) => output.validates_kernel_eof_authority(authority, version),
            Self::Exclusive(output) => output.validates_kernel_eof_authority(authority, version),
            #[cfg(test)]
            Self::Scripted(output) => output.validates_kernel_eof_authority(authority, version),
        }
    }

    pub(crate) fn commit_if_kernel_eof_current(
        &mut self,
        authority: &mut KernelPtyEofAuthority,
        version: &ProcessClosureVersion,
        operation: impl FnOnce(
            &mut dyn FnMut(KernelEofValidationRequest) -> Result<Option<KernelEofValidation>>,
        ) -> Result<KernelEofCommitOutcome>,
    ) -> Result<bool> {
        match self {
            Self::Reader(output) => {
                output.commit_if_kernel_eof_current(authority, version, |validate| {
                    operation(validate)
                })
            }
            Self::Exclusive(output) => {
                output.commit_if_kernel_eof_current(authority, version, |validate| {
                    operation(validate)
                })
            }
            #[cfg(test)]
            Self::Scripted(output) => {
                if !output.validates_kernel_eof_authority(authority, version)? {
                    return Ok(false);
                }
                let mut revalidate = |request| match request {
                    KernelEofValidationRequest::Current => output
                        .validate_kernel_eof_authority(authority, version)
                        .map(|validated| validated.map(KernelEofValidation::Current)),
                    KernelEofValidationRequest::ReopenSealed { validated, seal } => Ok(authority
                        .bind_committed_reopen_seal(validated, &seal)
                        .map(KernelEofValidation::ReopenSealed)),
                };
                let outcome = operation(&mut revalidate)?;
                drop(revalidate);
                Ok(output.apply_kernel_eof_outcome(authority, outcome))
            }
        }
    }
}
