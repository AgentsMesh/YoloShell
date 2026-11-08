use super::{AcceptedIngressExchange, PendingIngressExchange, read_optional};
use crate::agent::hooks::{EventSequence, HookEnvelope, HookProviderKind, Reducer};

const SINGLE_OCCURRENCE_RECEIPT: usize = 1;
#[derive(Clone, Copy)]
pub(super) enum CleanupExpectation {
    Complete,
    Retained,
}

impl PendingIngressExchange {
    pub(in crate::worker::codex_hook::tests) fn is_running(&mut self) -> bool {
        self.client.is_running()
    }

    pub(in crate::worker::codex_hook::tests) fn assert_committed(&self) -> EventSequence {
        assert_committed(
            &self.intake,
            &self.outbox,
            &self.snapshot_path,
            &self.occurrence,
            &self.before_outbox,
            self.before_snapshot.as_deref(),
            CleanupExpectation::Retained,
        )
    }

    pub(in crate::worker::codex_hook::tests) async fn finish_accepted(
        self,
    ) -> AcceptedIngressExchange {
        let Self {
            client,
            intake,
            outbox,
            snapshot_path,
            occurrence,
            before_outbox,
            before_snapshot,
        } = self;
        let ack = client.finish().await.unwrap();
        assert!(ack.accepted);
        assert_eq!(ack.code, crate::codex_hooks::ACK_CODE_ACCEPTED);
        assert_eq!(ack.event_id, Some(occurrence.event_id));
        let assigned = ack.sequence.unwrap();
        assert!(assigned >= crate::agent::hooks::FIRST_SEQUENCE);
        assert_eq!(
            assert_committed(
                &intake,
                &outbox,
                &snapshot_path,
                &occurrence,
                &before_outbox,
                before_snapshot.as_deref(),
                CleanupExpectation::Complete,
            ),
            EventSequence(assigned)
        );
        AcceptedIngressExchange { ack, occurrence }
    }

    pub(in crate::worker::codex_hook::tests) async fn finish_rejected(
        self,
    ) -> super::super::super::super::protocol::CodexHookAck {
        let Self {
            client,
            intake,
            outbox,
            snapshot_path,
            occurrence,
            before_outbox,
            before_snapshot,
        } = self;
        let ack = client.finish().await.unwrap();
        assert!(!ack.accepted);
        assert_unchanged(
            &intake,
            &outbox,
            &snapshot_path,
            &occurrence,
            before_outbox,
            before_snapshot,
        );
        ack
    }

    pub(in crate::worker::codex_hook::tests) async fn finish_without_ack(self) -> HookEnvelope {
        let Self {
            client,
            intake,
            outbox,
            snapshot_path,
            occurrence,
            before_outbox,
            before_snapshot,
        } = self;
        assert!(client.finish().await.is_err());
        assert_unchanged(
            &intake,
            &outbox,
            &snapshot_path,
            &occurrence,
            before_outbox,
            before_snapshot,
        );
        occurrence
    }

    pub(in crate::worker::codex_hook::tests) async fn finish_aborted(self) -> EventSequence {
        let Self {
            client,
            intake,
            outbox,
            snapshot_path,
            occurrence,
            before_outbox,
            before_snapshot,
        } = self;
        assert!(client.finish().await.is_err());
        assert_committed(
            &intake,
            &outbox,
            &snapshot_path,
            &occurrence,
            &before_outbox,
            before_snapshot.as_deref(),
            CleanupExpectation::Retained,
        )
    }
}

pub(super) fn assert_committed(
    intake: &crate::codex_hooks::CodexHookIntake,
    outbox: &crate::codex_hooks::CodexOutbox,
    snapshot_path: &std::path::Path,
    occurrence: &crate::agent::hooks::HookEnvelope,
    before_outbox: &crate::codex_hooks::CodexOutboxState,
    before_snapshot: Option<&[u8]>,
    cleanup: CleanupExpectation,
) -> EventSequence {
    let state = outbox.load().unwrap();
    assert_ne!(&state, before_outbox);
    assert!(state.pending_for_test().is_empty());
    let assigned = EventSequence(state.acknowledged_sequence());
    assert!(assigned >= EventSequence(crate::agent::hooks::FIRST_SEQUENCE));
    let mut expected = occurrence.clone();
    expected.sequence = assigned;
    assert_eq!(state.acknowledged_events_for_test().back(), Some(&expected));
    match cleanup {
        CleanupExpectation::Complete => {
            assert!(state.occurrence_receipt_count_for_test() == usize::default());
            assert!(intake.find_exact_occurrence(occurrence).unwrap().is_none());
        }
        CleanupExpectation::Retained => {
            let durable = intake
                .find_exact_occurrence(occurrence)
                .unwrap()
                .expect("raw occurrence retained until acknowledgement settlement");
            let identity = durable.durable_identity().unwrap();
            assert_eq!(
                state.occurrence_receipt_count_for_test(),
                SINGLE_OCCURRENCE_RECEIPT
            );
            assert!(
                state
                    .proves_occurrence_receipt(&expected, &identity)
                    .unwrap()
            );
        }
    }
    let after_snapshot = read_optional(snapshot_path);
    assert!(
        after_snapshot
            .as_deref()
            .is_some_and(|after| { before_snapshot.is_none_or(|before| after != before) })
    );
    let (_, snapshot) = crate::codex_hooks::CodexSnapshotCursor::open(snapshot_path).unwrap();
    let reducer = Reducer::restore(
        snapshot.unwrap(),
        occurrence.identity.clone(),
        HookProviderKind::Codex,
        occurrence.provider_version,
    )
    .unwrap();
    assert_eq!(reducer.state().last_sequence, assigned);
    assigned
}

fn assert_unchanged(
    intake: &crate::codex_hooks::CodexHookIntake,
    outbox: &crate::codex_hooks::CodexOutbox,
    snapshot_path: &std::path::Path,
    occurrence: &crate::agent::hooks::HookEnvelope,
    before_outbox: crate::codex_hooks::CodexOutboxState,
    before_snapshot: Option<Vec<u8>>,
) {
    assert_eq!(outbox.load().unwrap(), before_outbox);
    assert_eq!(read_optional(snapshot_path), before_snapshot);
    assert!(intake.find_exact_occurrence(occurrence).unwrap().is_some());
}
