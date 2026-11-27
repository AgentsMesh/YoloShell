use super::{
    FinalizedProducersGoneReady, FrozenGoneAuthority, FrozenProcessTree,
    FrozenProducersGoneFinalization, FrozenProducersGoneReady,
};
use crate::process::CommittedPtyReopenSealAuthority;
use crate::process::captured_tree::{gone, transition::ProcessTransitionError};
use crate::process::kernel_eof_authority::ValidatedSealedKernelEof;
use anyhow::Context;

const KERNEL_EOF_GONE_AUTHORITY_MISMATCH_DETAIL: &str =
    "kernel EOF validation belongs to another producers-gone transaction";
const FINALIZED_GONE_TRANSACTION_MISMATCH_DETAIL: &str =
    "finalized producers-gone token belongs to another process-tree transaction";

impl FrozenProcessTree {
    pub fn validate_producers_gone_ready(
        &self,
        ready: &FrozenProducersGoneReady,
    ) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.ready_matches(ready)
                && self
                    .producers_gone_transaction
                    .as_ref()
                    .is_some_and(|transaction| transaction.matches_unfinalized(&ready.transaction)),
            super::empty::GONE_PROOF_VERSION_MISMATCH_DETAIL
        );
        Ok(())
    }

    pub fn finalize_producers_gone_commit(
        &mut self,
        ready: &FrozenProducersGoneReady,
        validated: Option<ValidatedSealedKernelEof>,
    ) -> anyhow::Result<FrozenProducersGoneFinalization> {
        anyhow::ensure!(
            self.ready_matches(ready)
                && self.producers_gone_transaction.as_ref().is_some_and(
                    |transaction| transaction.matches_commit_prepared(&ready.transaction)
                ),
            super::empty::GONE_PROOF_VERSION_MISMATCH_DETAIL
        );
        let authority = final_output_authority(ready, validated)
            .context(KERNEL_EOF_GONE_AUTHORITY_MISMATCH_DETAIL)?;
        match &authority {
            FinalOutputAuthority::GlobalExclusion => {}
            FinalOutputAuthority::KernelEof(seal) => {
                self.revalidate_kernel_eof_reopen_seal(seal)?;
            }
        }
        let current = match self.gone_process_authority_is_current(ready.authority) {
            Ok(current) => current,
            Err(source) => {
                self.invalidate_producers_gone_commit();
                return Err(super::empty_commit_state::post_seal_error(
                    ready.authority,
                    source,
                ));
            }
        };
        if !current {
            self.invalidate_producers_gone_commit();
            return Ok(match ready.authority {
                FrozenGoneAuthority::GlobalExclusion => FrozenProducersGoneFinalization::Refreeze,
                FrozenGoneAuthority::KernelEof => {
                    FrozenProducersGoneFinalization::OutputInvalidated
                }
            });
        }
        // The helper itself requires independent premature-release coverage; at this point a
        // Kernel EOF authority, when present, has already passed its final revalidation.
        self.tree
            .commit_confirmed_bootstrap_global_exclusion_if_authorized();
        self.producers_gone_authority = Some(ready.authority);
        self.refreeze_required = false;
        let transaction = self
            .producers_gone_transaction
            .as_mut()
            .expect(super::empty::GONE_PROOF_VERSION_MISMATCH_DETAIL);
        assert!(
            transaction.finalize(&ready.transaction),
            "{}",
            super::empty::GONE_PROOF_VERSION_MISMATCH_DETAIL
        );
        Ok(FrozenProducersGoneFinalization::Finalized(finalized_ready(
            ready,
        )))
    }

    pub fn commit_producers_gone(
        self,
        ready: FinalizedProducersGoneReady,
    ) -> Result<gone::TrackedProducersGone, ProcessTransitionError<Self>> {
        let authorized = self.version == ready.version
            && self.producers_gone_authority == Some(ready.authority)
            && !self.refreeze_required
            && self.closure.is_empty()
            && self
                .producers_gone_transaction
                .as_ref()
                .is_some_and(|transaction| transaction.matches_finalized(&ready.transaction));
        if !authorized {
            return Err(ProcessTransitionError::new(
                self,
                anyhow::anyhow!(FINALIZED_GONE_TRANSACTION_MISMATCH_DETAIL),
            ));
        }
        Ok(gone::complete(self.tree))
    }

    fn ready_matches(&self, ready: &FrozenProducersGoneReady) -> bool {
        self.version == ready.version
            && self.producers_gone_authority == Some(ready.authority)
            && ready_authority_is_well_formed(ready)
    }
}

fn finalized_ready(ready: &FrozenProducersGoneReady) -> FinalizedProducersGoneReady {
    FinalizedProducersGoneReady {
        version: ready.version.clone(),
        authority: ready.authority,
        transaction: ready.transaction.clone(),
    }
}

fn ready_authority_is_well_formed(ready: &FrozenProducersGoneReady) -> bool {
    match ready.authority {
        FrozenGoneAuthority::GlobalExclusion => ready.kernel_eof.is_none(),
        FrozenGoneAuthority::KernelEof => ready.kernel_eof.is_some(),
    }
}

enum FinalOutputAuthority {
    GlobalExclusion,
    KernelEof(CommittedPtyReopenSealAuthority),
}

fn final_output_authority(
    ready: &FrozenProducersGoneReady,
    validated: Option<ValidatedSealedKernelEof>,
) -> Option<FinalOutputAuthority> {
    match (ready.authority, ready.kernel_eof.as_ref(), validated) {
        (FrozenGoneAuthority::GlobalExclusion, None, None) => {
            Some(FinalOutputAuthority::GlobalExclusion)
        }
        (FrozenGoneAuthority::KernelEof, Some(authority), Some(validated)) => authority
            .consume_sealed_token(validated)
            .map(FinalOutputAuthority::KernelEof),
        _ => None,
    }
}
