use super::{
    COMPLETION_RECEIVER_DROPPED_DETAIL, CompletionTarget, DEFERRED_REAP_FAILURE_DETAIL,
    FaultReapCompletion, PublicationState, ReapOutcome, log_outcome,
};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::atomic::Ordering;

pub(crate) const COMPLETION_OUTCOME_MISSING_DETAIL: &str =
    "PTY fault completion outcome is present";

impl FaultReapCompletion {
    pub(crate) fn publish(&self, outcome: ReapOutcome) {
        let mut pending = Some(outcome);
        if self
            .shared
            .published
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            dispose_outcome(pending);
            return;
        }
        match catch_unwind(AssertUnwindSafe(|| self.publish_target(&mut pending))) {
            Ok(()) => self.finish_publication(PublicationState::Complete),
            Err(payload) => self.publish_target_panic(payload),
        }
        dispose_outcome(pending);
    }

    pub(crate) fn publish_target(&self, pending: &mut Option<ReapOutcome>) {
        match &self.shared.target {
            CompletionTarget::Owner(sender) => {
                if let Err(error) = sender.send(take_outcome(pending)) {
                    log_outcome(error.0, COMPLETION_RECEIVER_DROPPED_DETAIL);
                }
            }
            CompletionTarget::Finalize(state) => crate::worker::pty_owner::complete_deferred_reap(
                Arc::clone(state),
                take_outcome(pending),
            ),
            CompletionTarget::Log => {
                log_outcome(take_outcome(pending), DEFERRED_REAP_FAILURE_DETAIL);
            }
            #[cfg(test)]
            CompletionTarget::Panic => panic!("{}", super::INJECTED_COMPLETION_TARGET_PANIC),
            #[cfg(test)]
            CompletionTarget::PanicOnDrop => super::panic::panic_on_drop_for_test(),
        }
    }
}

pub(crate) fn take_outcome(pending: &mut Option<ReapOutcome>) -> ReapOutcome {
    pending.take().expect(COMPLETION_OUTCOME_MISSING_DETAIL)
}

pub(crate) fn dispose_outcome(outcome: Option<ReapOutcome>) {
    let _ = crate::worker::supervision::contain(|| drop(outcome));
}
