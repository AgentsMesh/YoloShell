use super::{AcceptedIngressExchange, PendingIngressExchange};
use crate::agent::hooks::EventSequence;

impl PendingIngressExchange {
    pub(in crate::worker::codex_hook::tests) async fn finish_drained_without_ack(
        self,
    ) -> EventSequence {
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
        super::assertions::assert_committed(
            &intake,
            &outbox,
            &snapshot_path,
            &occurrence,
            &before_outbox,
            before_snapshot.as_deref(),
            super::assertions::CleanupExpectation::Complete,
        )
    }

    pub(in crate::worker::codex_hook::tests) async fn observe_accepted_pending_cleanup(
        self,
    ) -> (
        AcceptedIngressExchange,
        super::super::super::ingress_live_client::PendingPeerExchange,
    ) {
        let assigned = self.assert_committed();
        let occurrence = self.occurrence.clone();
        let mut client = self.client;
        let acknowledgement = client.observe().await.unwrap();

        assert!(acknowledgement.accepted);
        assert_eq!(acknowledgement.code, crate::codex_hooks::ACK_CODE_ACCEPTED);
        assert_eq!(acknowledgement.event_id, Some(occurrence.event_id));
        assert_eq!(acknowledgement.sequence, Some(assigned.0));
        assert!(assigned >= EventSequence(crate::agent::hooks::FIRST_SEQUENCE));
        (
            AcceptedIngressExchange {
                ack: acknowledgement,
                occurrence,
            },
            client,
        )
    }
}
