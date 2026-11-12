use super::{FrozenGoneAuthority, FrozenProcessTree, FrozenProducersGoneCommit};

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../../../integration_tests/captured/tree/force/captured_tree_force_empty_commit_state_test_hook.rs"]
mod test_hook;

impl FrozenProcessTree {
    pub(super) fn gone_process_authority_is_current(
        &mut self,
        authority: FrozenGoneAuthority,
    ) -> anyhow::Result<bool> {
        #[cfg(all(test, feature = "yolo-integration-tests"))]
        test_hook::fail_if_requested()?;
        if !self.closure.is_empty()
            || self.refreeze_required
            || !self.closure_remains_frozen_for(authority)?
        {
            return Ok(false);
        }
        Ok(match authority {
            FrozenGoneAuthority::GlobalExclusion => {
                super::empty::empty_closure_can_report_gone(self.tree.descendant_observation())
            }
            FrozenGoneAuthority::KernelEof => self.tree.frozen_closure_is_settled(
                super::super::frozen_authority::FrozenClosureAuthority::OutputFencedEmpty,
            ),
        })
    }

    pub(super) fn invalidate_producers_gone_commit(&mut self) {
        self.producers_gone_authority = None;
        self.producers_gone_transaction = None;
        self.refreeze_required = true;
    }
}

#[cfg(all(test, feature = "yolo-integration-tests"))]
impl FrozenProcessTree {
    pub(crate) fn fail_next_gone_authority_check_for_test(&mut self) {
        let _ = self;
        test_hook::fail_next();
    }
}

pub(super) fn post_seal_error(
    authority: FrozenGoneAuthority,
    source: anyhow::Error,
) -> anyhow::Error {
    match authority {
        FrozenGoneAuthority::GlobalExclusion => source,
        FrozenGoneAuthority::KernelEof => {
            FrozenProcessTree::kernel_eof_output_invalidating_error(source)
        }
    }
}

pub(super) fn post_seal_refreeze(authority: FrozenGoneAuthority) -> FrozenProducersGoneCommit {
    match authority {
        FrozenGoneAuthority::GlobalExclusion => FrozenProducersGoneCommit::Refreeze,
        FrozenGoneAuthority::KernelEof => FrozenProducersGoneCommit::FreshEofRequired,
    }
}
