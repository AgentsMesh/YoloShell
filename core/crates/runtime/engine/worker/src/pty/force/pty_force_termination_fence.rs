use super::{
    EmptyClosureOutputRequirement, EmptyGoneOutputAuthority, ForceOutput, ForceReady,
    FrozenGoneReady, OutputFence, PtyForceTermination, VersionedOutputFence,
};
use crate::process::{
    FrozenProcessTree, FrozenProducersGonePoll, KernelEofKillPreparation, KillCommitReady,
};
use crate::worker::pty_output_progress::{
    KernelEofCommitOutcome, KernelEofValidation, KernelEofValidationRequest,
};
use anyhow::{Context, Result};

pub(crate) const OUTPUT_FENCE_DISAPPEARED_DETAIL: &str =
    "versioned PTY output fence disappeared before polling";
pub(crate) const KERNEL_EOF_KILL_READY_DETAIL: &str =
    "kernel EOF kill transaction completed without a prepared transition";
pub(crate) const KERNEL_EOF_GONE_PERMIT_MISSING_DETAIL: &str =
    "kernel EOF observation did not provide a producers-gone permit";

impl PtyForceTermination {
    pub(crate) fn prepare_frozen(
        &mut self,
        tree: &mut FrozenProcessTree,
        output: &mut ForceOutput<'_>,
    ) -> Result<ForceReady> {
        if self.frozen_gone.is_some() {
            return Ok(ForceReady::FrozenGone);
        }
        if self.empty_closure_output_requirement(tree) == EmptyClosureOutputRequirement::KernelEof {
            self.output_fence = None;
            return self.prepare_pty_empty_gone(tree, output);
        }
        match tree.prepare_producers_gone()? {
            FrozenProducersGonePoll::NotEmpty => {}
            FrozenProducersGonePoll::Pending => return Ok(ForceReady::Unchanged),
            FrozenProducersGonePoll::Refreeze(ready) => {
                self.output_fence = None;
                return Ok(ForceReady::Frozen(ready));
            }
            FrozenProducersGonePoll::Gone(ready) => {
                self.output_fence = None;
                self.frozen_gone = Some(FrozenGoneReady {
                    process: ready,
                    output: EmptyGoneOutputAuthority::NotRequired,
                });
                return Ok(ForceReady::FrozenGone);
            }
        }
        let version = tree.closure_version();
        if self
            .output_fence
            .as_ref()
            .is_some_and(|fence| fence.version != version)
        {
            self.output_fence = None;
        }
        if self.output_fence.is_none() {
            let state = output.begin_fence()?;
            self.output_fence = Some(VersionedOutputFence {
                version,
                state,
                kernel_eof: None,
            });
            self.mark_progress();
            return Ok(ForceReady::Unchanged);
        }

        let (progressed, reached) = {
            let fence = self
                .output_fence
                .as_mut()
                .expect(OUTPUT_FENCE_DISAPPEARED_DETAIL);
            let previous = fence.state.progress();
            if !matches!(fence.state, OutputFence::Reached) {
                output.poll_fence(&mut fence.state)?;
            }
            (
                fence.state.progress() != previous,
                matches!(fence.state, OutputFence::Reached),
            )
        };
        if progressed {
            self.mark_progress();
        }
        if !reached {
            return Ok(ForceReady::Unchanged);
        }
        let ready = prepare_fenced_kill(
            tree,
            output,
            self.output_fence
                .as_mut()
                .expect(OUTPUT_FENCE_DISAPPEARED_DETAIL),
        )?;
        self.output_fence = None;
        Ok(ready)
    }
    pub(crate) fn prepare_pty_empty_gone(
        &mut self,
        tree: &mut FrozenProcessTree,
        output: &mut ForceOutput<'_>,
    ) -> Result<ForceReady> {
        let closure_stable = tree.empty_closure_remains_frozen_with_kernel_eof()?;
        if !closure_stable {
            return Ok(ForceReady::Frozen(KillCommitReady::Refreeze));
        }
        let version = tree.closure_version();
        let authority = output.acquire_kernel_eof_authority(&version)?;
        let Some(mut authority) = authority else {
            return Ok(ForceReady::Unchanged);
        };
        let authority_valid = output.validates_kernel_eof_authority(&authority, &version)?;
        if !authority_valid {
            return Ok(ForceReady::Unchanged);
        }
        let permit = authority
            .take_permit()
            .context(KERNEL_EOF_GONE_PERMIT_MISSING_DETAIL)?;
        match tree.prepare_producers_gone_with_kernel_eof(permit)? {
            FrozenProducersGonePoll::NotEmpty => Ok(ForceReady::Frozen(KillCommitReady::Refreeze)),
            FrozenProducersGonePoll::Pending => Ok(ForceReady::Unchanged),
            FrozenProducersGonePoll::Refreeze(ready) => Ok(ForceReady::Frozen(ready)),
            FrozenProducersGonePoll::Gone(ready) => {
                if !output.validates_kernel_eof_authority(&authority, &tree.closure_version())? {
                    return Ok(ForceReady::Unchanged);
                }
                self.frozen_gone = Some(FrozenGoneReady {
                    process: ready,
                    output: EmptyGoneOutputAuthority::Kernel(authority),
                });
                Ok(ForceReady::FrozenGone)
            }
        }
    }
}

pub(crate) fn prepare_fenced_kill(
    tree: &mut FrozenProcessTree,
    output: &mut ForceOutput<'_>,
    fence: &mut VersionedOutputFence,
) -> Result<ForceReady> {
    let version = fence.version.clone();
    let Some(authority) = fence.kernel_eof.as_mut() else {
        return tree.prepare_kill(version).map(ForceReady::Frozen);
    };
    let mut prepared = None;
    let committed = output.commit_if_kernel_eof_current(authority, &version, |validate| {
        let Some(KernelEofValidation::Current(validated)) =
            validate(KernelEofValidationRequest::Current)?
        else {
            return Ok(KernelEofCommitOutcome::Rejected);
        };
        let ready = match tree.prepare_kill_with_kernel_eof(version.clone())? {
            KernelEofKillPreparation::Refreeze => {
                return Ok(KernelEofCommitOutcome::Rejected);
            }
            KernelEofKillPreparation::FreshEofRequired => {
                return Ok(KernelEofCommitOutcome::OutputInvalidated);
            }
            KernelEofKillPreparation::Ready(ready) => ready,
        };
        let seal = FrozenProcessTree::kernel_eof_reopen_seal(&ready).clone();
        let Some(KernelEofValidation::ReopenSealed(validated)) =
            validate(KernelEofValidationRequest::ReopenSealed { validated, seal })?
        else {
            return Ok(KernelEofCommitOutcome::Rejected);
        };
        prepared = Some(tree.commit_kill_with_kernel_eof(ready, validated)?);
        Ok(KernelEofCommitOutcome::Committed)
    })?;
    if !committed {
        return Ok(ForceReady::Frozen(KillCommitReady::Refreeze));
    }
    prepared
        .map(ForceReady::Frozen)
        .context(KERNEL_EOF_KILL_READY_DETAIL)
}
