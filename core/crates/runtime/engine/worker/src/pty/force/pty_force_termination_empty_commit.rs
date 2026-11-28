use super::{EmptyGoneOutputAuthority, ForceOutput, ForceTerminationPoll, PtyForceTermination};
use crate::process::kernel_eof_authority::{ValidatedKernelEof, ValidatedSealedKernelEof};
use crate::process::{
    CommittedPtyReopenSealAuthority, FrozenProcessTree, FrozenProducersGoneCommit,
    FrozenProducersGoneFinalization,
};
use crate::worker::pty_output_progress::{
    KernelEofCommitOutcome, KernelEofValidation, KernelEofValidationRequest,
};
use crate::worker::pty_process_custody::PtyProcessCustody;
use anyhow::{Context, Result};

pub(crate) const FROZEN_GONE_TRANSACTION_MISSING_DETAIL: &str =
    "prepared frozen-gone transaction is unavailable";

impl PtyForceTermination {
    pub(crate) fn commit_frozen_gone(
        &mut self,
        custody: &mut Option<PtyProcessCustody>,
        output: &mut ForceOutput<'_>,
    ) -> Result<ForceTerminationPoll> {
        let ready = self
            .frozen_gone
            .as_mut()
            .context(FROZEN_GONE_TRANSACTION_MISSING_DETAIL)?;
        let version = {
            let tree = frozen_tree(custody);
            tree.validate_producers_gone_ready(&ready.process)?;
            tree.closure_version()
        };
        let process = &ready.process;
        let committed = match &mut ready.output {
            EmptyGoneOutputAuthority::NotRequired => {
                commit_prepared_gone(custody, process, None, None)
                    .map(|outcome| outcome == KernelEofCommitOutcome::Committed)
            }
            EmptyGoneOutputAuthority::Kernel(authority) => {
                output.commit_if_kernel_eof_current(authority, &version, |validate| {
                    let Some(KernelEofValidation::Current(validated)) =
                        validate(KernelEofValidationRequest::Current)?
                    else {
                        return Ok(KernelEofCommitOutcome::Rejected);
                    };
                    let mut revalidate = |validated, seal| match validate(
                        KernelEofValidationRequest::ReopenSealed { validated, seal },
                    )? {
                        Some(KernelEofValidation::ReopenSealed(validated)) => Ok(Some(validated)),
                        Some(KernelEofValidation::Current(_)) | None => Ok(None),
                    };
                    commit_prepared_gone(custody, process, Some(validated), Some(&mut revalidate))
                })
            }
        };
        if committed.is_ok() {
            self.frozen_gone = None;
        }
        committed?;
        Ok(ForceTerminationPoll::Pending)
    }
}

pub(crate) fn commit_prepared_gone(
    custody: &mut Option<PtyProcessCustody>,
    ready: &crate::process::FrozenProducersGoneReady,
    validated: Option<ValidatedKernelEof>,
    mut revalidate: Option<&mut SealOutputRevalidation<'_>>,
) -> Result<KernelEofCommitOutcome> {
    let preparation = frozen_tree_mut(custody).prepare_producers_gone_commit(ready)?;
    let sealed = match preparation {
        FrozenProducersGoneCommit::Refreeze => return Ok(KernelEofCommitOutcome::Rejected),
        FrozenProducersGoneCommit::FreshEofRequired => {
            return Ok(KernelEofCommitOutcome::OutputInvalidated);
        }
        FrozenProducersGoneCommit::Ready if validated.is_none() => None,
        FrozenProducersGoneCommit::Ready => return Ok(KernelEofCommitOutcome::Rejected),
        FrozenProducersGoneCommit::ReopenSealed(seal) => {
            let Some(validated) = validated else {
                return Ok(KernelEofCommitOutcome::Rejected);
            };
            let sealed = revalidate_output(&mut revalidate, validated, seal)?;
            let Some(sealed) = sealed else {
                return Ok(KernelEofCommitOutcome::Rejected);
            };
            Some(sealed)
        }
    };
    let finalized = match frozen_tree_mut(custody).finalize_producers_gone_commit(ready, sealed)? {
        FrozenProducersGoneFinalization::Finalized(finalized) => finalized,
        FrozenProducersGoneFinalization::OutputInvalidated => {
            return Ok(KernelEofCommitOutcome::OutputInvalidated);
        }
        FrozenProducersGoneFinalization::Refreeze => {
            return Ok(KernelEofCommitOutcome::Rejected);
        }
    };
    let state = custody
        .take()
        .expect(super::commit::PROCESS_CUSTODY_COMMIT_MISSING_DETAIL);
    let PtyProcessCustody::Frozen(tree) = state else {
        *custody = Some(state);
        panic!("{}", super::commit::PROCESS_CUSTODY_VARIANT_CHANGED_DETAIL);
    };
    let gone = tree.commit_producers_gone(finalized).map_err(|failure| {
        let (tree, error) = failure.into_parts();
        *custody = Some(PtyProcessCustody::Frozen(tree));
        error
    })?;
    *custody = Some(PtyProcessCustody::GonePending(gone));
    Ok(KernelEofCommitOutcome::Committed)
}

pub(crate) type SealOutputRevalidation<'a> = dyn FnMut(
        ValidatedKernelEof,
        CommittedPtyReopenSealAuthority,
    ) -> Result<Option<ValidatedSealedKernelEof>>
    + 'a;

pub(crate) fn revalidate_output(
    revalidate: &mut Option<&mut SealOutputRevalidation<'_>>,
    validated: ValidatedKernelEof,
    seal: CommittedPtyReopenSealAuthority,
) -> Result<Option<ValidatedSealedKernelEof>> {
    match revalidate {
        Some(operation) => operation(validated, seal),
        None => Ok(None),
    }
}

pub(crate) fn frozen_tree(custody: &Option<PtyProcessCustody>) -> &FrozenProcessTree {
    match custody.as_ref() {
        Some(PtyProcessCustody::Frozen(tree)) => tree,
        Some(_) => panic!("{}", super::commit::PROCESS_CUSTODY_VARIANT_CHANGED_DETAIL),
        None => panic!("{}", super::commit::PROCESS_CUSTODY_COMMIT_MISSING_DETAIL),
    }
}

pub(crate) fn frozen_tree_mut(custody: &mut Option<PtyProcessCustody>) -> &mut FrozenProcessTree {
    match custody.as_mut() {
        Some(PtyProcessCustody::Frozen(tree)) => tree,
        Some(_) => panic!("{}", super::commit::PROCESS_CUSTODY_VARIANT_CHANGED_DETAIL),
        None => panic!("{}", super::commit::PROCESS_CUSTODY_COMMIT_MISSING_DETAIL),
    }
}
