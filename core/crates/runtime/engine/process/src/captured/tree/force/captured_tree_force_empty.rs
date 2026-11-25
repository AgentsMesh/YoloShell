use super::empty_commit_state::{post_seal_error, post_seal_refreeze};
use super::{
    FrozenGoneAuthority, FrozenProcessTree, FrozenProducersGoneCommit, FrozenProducersGonePoll,
    FrozenProducersGoneReady, KernelEofReopenSealCommit, KillCommitReady,
};
use crate::process::DescendantObservation;

pub(super) const GONE_PROOF_VERSION_MISMATCH_DETAIL: &str =
    "producer-gone proof belongs to another process-tree closure";
impl FrozenProcessTree {
    pub fn empty_gone_requires_output_eof(&self) -> bool {
        self.closure.is_empty() && self.tree.empty_output_fence_is_authorized()
    }

    pub fn pty_master_released(&self) -> bool {
        self.tree.pty_master_released()
    }

    pub fn empty_closure_remains_frozen_with_kernel_eof(&mut self) -> anyhow::Result<bool> {
        if !self.closure.is_empty() || self.refreeze_required {
            return Ok(false);
        }
        super::kernel_eof::closure_remains_frozen(self)
    }

    pub fn prepare_producers_gone(&mut self) -> anyhow::Result<FrozenProducersGonePoll> {
        self.prepare_producers_gone_with(FrozenGoneAuthority::GlobalExclusion, None)
    }

    pub fn prepare_producers_gone_with_kernel_eof(
        &mut self,
        permit: crate::process::kernel_eof_authority::KernelEofPermit,
    ) -> anyhow::Result<FrozenProducersGonePoll> {
        let Some(authority) = permit.authorize(&self.version) else {
            return Ok(FrozenProducersGonePoll::Refreeze(KillCommitReady::Refreeze));
        };
        self.prepare_producers_gone_with(FrozenGoneAuthority::KernelEof, Some(authority))
    }

    fn prepare_producers_gone_with(
        &mut self,
        authority: FrozenGoneAuthority,
        kernel_eof: Option<crate::process::kernel_eof_authority::KernelEofCommitBinding>,
    ) -> anyhow::Result<FrozenProducersGonePoll> {
        if !self.closure.is_empty() {
            return Ok(FrozenProducersGonePoll::NotEmpty);
        }
        if self.refreeze_required {
            return Ok(FrozenProducersGonePoll::Refreeze(KillCommitReady::Refreeze));
        }
        if let Some(prepared) = self.producers_gone_authority {
            if prepared == authority {
                return Ok(self.prepared_gone(None, prepared, kernel_eof));
            }
            self.producers_gone_authority = None;
            self.producers_gone_transaction = None;
        }
        if !self.closure_remains_frozen_for(authority)? {
            return Ok(FrozenProducersGonePoll::Refreeze(KillCommitReady::Refreeze));
        }
        let authorized = match authority {
            FrozenGoneAuthority::GlobalExclusion => {
                empty_closure_can_report_gone(self.tree.descendant_observation())
            }
            FrozenGoneAuthority::KernelEof => self.tree.frozen_closure_is_settled(
                super::super::frozen_authority::FrozenClosureAuthority::OutputFencedEmpty,
            ),
        };
        if !authorized {
            return Ok(FrozenProducersGonePoll::Pending);
        }
        #[cfg(target_os = "linux")]
        if self.tree.linux_containment_available() {
            use crate::process::linux_containment::LinuxContainmentPreparation;
            return Ok(match self.tree.prepare_linux_containment()? {
                LinuxContainmentPreparation::MembershipChanged => {
                    FrozenProducersGonePoll::Refreeze(KillCommitReady::Refreeze)
                }
                LinuxContainmentPreparation::Removed => {
                    self.producers_gone_authority = Some(authority);
                    self.prepared_gone(None, authority, kernel_eof)
                }
                LinuxContainmentPreparation::Retained(ready) => {
                    self.producers_gone_authority = Some(authority);
                    self.prepared_gone(Some(ready), authority, kernel_eof)
                }
            });
        }
        self.producers_gone_authority = Some(authority);
        Ok(self.prepared_gone(None, authority, kernel_eof))
    }
    pub fn prepare_producers_gone_commit(
        &mut self,
        ready: &FrozenProducersGoneReady,
    ) -> anyhow::Result<FrozenProducersGoneCommit> {
        self.validate_producers_gone_ready(ready)?;
        anyhow::ensure!(
            self.producers_gone_transaction
                .as_mut()
                .is_some_and(|transaction| transaction.prepare_commit(&ready.transaction)),
            GONE_PROOF_VERSION_MISMATCH_DETAIL
        );
        let reopen_seal = match ready.authority {
            FrozenGoneAuthority::GlobalExclusion => None,
            FrozenGoneAuthority::KernelEof => Some(self.prepare_kernel_eof_reopen_seal()?),
        };
        let current = match self.gone_process_authority_is_current(ready.authority) {
            Ok(current) => current,
            Err(source) => {
                self.invalidate_producers_gone_commit();
                return Err(post_seal_error(ready.authority, source));
            }
        };
        if !current {
            self.invalidate_producers_gone_commit();
            return Ok(post_seal_refreeze(ready.authority));
        }
        #[cfg(target_os = "linux")]
        if let Some(containment) = ready.retained.as_ref() {
            self.refreeze_required = true;
            let current = match self.tree.validate_linux_retained(containment) {
                Ok(current) => current,
                Err(source) => {
                    self.invalidate_producers_gone_commit();
                    return Err(post_seal_error(ready.authority, source));
                }
            };
            if !current {
                self.invalidate_producers_gone_commit();
                return Ok(post_seal_refreeze(ready.authority));
            }
            self.refreeze_required = false;
        }
        Ok(match reopen_seal {
            None => FrozenProducersGoneCommit::Ready,
            Some(seal) => match seal {
                KernelEofReopenSealCommit::FreshEofRequired => {
                    FrozenProducersGoneCommit::FreshEofRequired
                }
                KernelEofReopenSealCommit::Reusable(seal) => {
                    FrozenProducersGoneCommit::ReopenSealed(seal)
                }
            },
        })
    }
    fn prepared_gone(
        &mut self,
        #[cfg(target_os = "linux")] retained: Option<
            crate::process::linux_containment::LinuxContainmentCommitReady,
        >,
        #[cfg(not(target_os = "linux"))] _retained: Option<()>,
        authority: FrozenGoneAuthority,
        kernel_eof: Option<crate::process::kernel_eof_authority::KernelEofCommitBinding>,
    ) -> FrozenProducersGonePoll {
        let transaction = super::empty_transaction::ProducersGoneTransaction::fresh();
        self.producers_gone_transaction = Some(
            super::empty_transaction::ProducersGoneTransactionState::prepared(transaction.clone()),
        );
        FrozenProducersGonePoll::Gone(FrozenProducersGoneReady {
            version: self.version.clone(),
            authority,
            transaction,
            kernel_eof,
            #[cfg(target_os = "linux")]
            retained,
        })
    }

    pub(super) fn closure_remains_frozen_for(
        &mut self,
        authority: FrozenGoneAuthority,
    ) -> anyhow::Result<bool> {
        match authority {
            FrozenGoneAuthority::GlobalExclusion => super::commit::closure_remains_frozen(self),
            FrozenGoneAuthority::KernelEof => super::kernel_eof::closure_remains_frozen(self),
        }
    }
}

pub(super) fn empty_closure_can_report_gone(observation: DescendantObservation) -> bool {
    observation == DescendantObservation::Settled
}

#[cfg(any(all(test, feature = "yolo-integration-tests"), feature = "runtime-component-tests"))]
#[path = "../../../../test_support/captured_tree/captured_tree_force_empty.rs"]
mod test_support;
#[cfg(any(all(test, feature = "yolo-integration-tests"), feature = "runtime-component-tests"))]
pub(super) use test_support::assert_gone_proof_version;
