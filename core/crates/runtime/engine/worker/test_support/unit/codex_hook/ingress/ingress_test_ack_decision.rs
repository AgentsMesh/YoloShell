use super::{CleanupDecision, CleanupEvidence};
use crate::agent::hooks::HookViolation;

const SINGLE_DURABLE_REJECTION: usize = 1;

impl CleanupEvidence {
    pub(super) fn assert_decision(&self) {
        let state = self.outbox.load().unwrap();
        if matches!(self.decision, CleanupDecision::Accepted) {
            assert!(state.durable_rejections_for_test().is_empty());
            return;
        }
        assert_eq!(
            state.durable_rejections_for_test().len(),
            SINGLE_DURABLE_REJECTION
        );
        let marker = state
            .durable_rejections_for_test()
            .back()
            .expect("durable rejection marker");
        assert_eq!(marker.event_id(), self.event.event_id);
        assert_eq!(marker.sequence(), self.event.sequence);
        assert_eq!(marker.violation(), HookViolation::ProviderMismatch);
    }
}
