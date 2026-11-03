use super::{DurableRejectionExchange, PendingIngressExchange, read_optional};
use crate::agent::hooks::EventSequence;

const SINGLE_DURABLE_REJECTION_MARKER: usize = 1;

impl PendingIngressExchange {
    pub(in crate::worker::codex_hook::tests) async fn finish_durable_rejection(
        self,
    ) -> DurableRejectionExchange {
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
        assert_ne!(outbox.load().unwrap(), before_outbox);

        let sequence = EventSequence(ack.sequence.expect("durable rejection sequence"));
        let event_id = ack.event_id.expect("durable rejection event ID");
        let mut event = occurrence.clone();
        event.sequence = sequence;
        assert_eq!(event.event_id, event_id);
        assert_eq!(ack.event_id, Some(event.event_id));
        assert_eq!(ack.sequence, Some(event.sequence.0));
        assert_eq!(event.identity, occurrence.identity);

        let state = outbox.load().unwrap();
        assert_eq!(state.acknowledged_sequence(), sequence.0);
        assert!(state.pending_for_test().is_empty());
        assert!(state.pending_occurrence_witness_count_for_test() == usize::default());
        assert!(state.occurrence_receipt_count_for_test() == usize::default());
        assert_eq!(
            state.durable_rejections_for_test().len(),
            SINGLE_DURABLE_REJECTION_MARKER
        );
        assert_eq!(state.acknowledged_events_for_test().back(), Some(&event));
        let marker = state
            .durable_rejections_for_test()
            .front()
            .expect("durable rejection marker");
        assert_eq!(marker.event_id(), event_id);
        assert_eq!(marker.sequence(), sequence);
        assert!(intake.find_exact_occurrence(&occurrence).unwrap().is_none());
        assert_ne!(read_optional(snapshot_path), before_snapshot);

        DurableRejectionExchange {
            ack,
            occurrence,
            event,
        }
    }
}
