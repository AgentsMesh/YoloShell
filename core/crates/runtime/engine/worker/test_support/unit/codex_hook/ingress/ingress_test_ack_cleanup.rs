use super::super::super::super::protocol::CodexHookAck;
use super::super::super::ingress_live_client::PendingPeerExchange;
use super::super::super::ingress_live_source::PeerEvidence;
use super::PendingIngressExchange;
use crate::agent::hooks::HookEnvelope;
use crate::codex_hooks::{CodexHookIntake, CodexOccurrenceIdentity, CodexOutbox};

const SINGLE_OCCURRENCE_RECEIPT: usize = 1;

#[path = "ingress_test_ack_decision.rs"]
mod decision_assertions;

#[derive(Clone, Copy)]
enum CleanupDecision {
    Accepted,
    ProviderMismatch,
}

pub(in crate::worker::codex_hook::tests) struct CleanupEvidence {
    pub(in crate::worker::codex_hook::tests) ack: Option<CodexHookAck>,
    pub(in crate::worker::codex_hook::tests) event: HookEnvelope,
    intake: CodexHookIntake,
    outbox: CodexOutbox,
    occurrence: HookEnvelope,
    identity: CodexOccurrenceIdentity,
    peer: Option<PendingPeerExchange>,
    decision: CleanupDecision,
}

impl PendingIngressExchange {
    pub(in crate::worker::codex_hook::tests) async fn observe_durable_ack(self) -> CleanupEvidence {
        let evidence =
            CleanupEvidence::capture(self, true, CleanupDecision::ProviderMismatch).await;
        let ack = evidence.ack.as_ref().expect("observed acknowledgement");
        assert!(!ack.accepted);
        assert_eq!(ack.event_id, Some(evidence.event.event_id));
        assert_eq!(ack.sequence, Some(evidence.event.sequence.0));
        evidence
    }

    pub(in crate::worker::codex_hook::tests) async fn abort_rejected_before_ack(
        self,
    ) -> CleanupEvidence {
        CleanupEvidence::capture(self, false, CleanupDecision::ProviderMismatch).await
    }

    pub(in crate::worker::codex_hook::tests) async fn abort_accepted_before_ack(
        self,
    ) -> CleanupEvidence {
        CleanupEvidence::capture(self, false, CleanupDecision::Accepted).await
    }
}

impl super::super::IngressEndpoint {
    pub(in crate::worker::codex_hook::tests) async fn retry_over_server(
        &self,
        server: &super::super::super::super::server::CodexHookServer,
        occurrence: &HookEnvelope,
        evidence: PeerEvidence,
    ) -> CodexHookAck {
        let client = PendingPeerExchange::begin(
            self.assets.live_source(),
            self.assets.hook_executable(),
            server.socket_path(),
            occurrence,
            evidence,
        );
        let stream = server.accept().await.unwrap().0;
        let (handled, acknowledgement) =
            tokio::join!(server.handle_stream(stream), client.finish());
        let (_, response) = handled.unwrap().into_parts();
        response.unwrap();
        acknowledgement.unwrap()
    }
}

impl CleanupEvidence {
    async fn capture(
        pending: PendingIngressExchange,
        observe: bool,
        decision: CleanupDecision,
    ) -> Self {
        let PendingIngressExchange {
            client,
            intake,
            outbox,
            occurrence,
            ..
        } = pending;
        let durable = intake
            .find_exact_occurrence(&occurrence)
            .unwrap()
            .expect("raw occurrence retained before acknowledgement cleanup");
        let identity = durable.durable_identity().unwrap();
        let state = outbox.load().unwrap();
        let event = state
            .acknowledged_events_for_test()
            .back()
            .expect("acknowledged assigned event")
            .clone();
        let mut client = client;
        let (ack, peer) = if observe {
            let acknowledgement = client.observe().await.unwrap();
            (Some(acknowledgement), Some(client))
        } else {
            drop(client);
            (None, None)
        };
        let evidence = Self {
            ack,
            event,
            intake,
            outbox,
            occurrence,
            identity,
            peer,
            decision,
        };
        evidence.assert_decision();
        evidence
    }

    pub(in crate::worker::codex_hook::tests) fn occurrence(&self) -> &HookEnvelope {
        &self.occurrence
    }

    pub(in crate::worker::codex_hook::tests) fn outbox_path(&self) -> std::path::PathBuf {
        self.outbox.path().to_path_buf()
    }

    pub(in crate::worker::codex_hook::tests) async fn finish_peer(&mut self) {
        if let Some(peer) = self.peer.take() {
            peer.wait_for_exit().await.unwrap();
        }
    }

    pub(in crate::worker::codex_hook::tests) fn assert_cleanup_pending(&self) {
        let occurrence = self
            .intake
            .find_exact_occurrence(&self.occurrence)
            .unwrap()
            .expect("raw occurrence retained");
        assert_eq!(occurrence.durable_identity().unwrap(), self.identity);
        let state = self.outbox.load().unwrap();
        assert_eq!(
            state.occurrence_receipt_count_for_test(),
            SINGLE_OCCURRENCE_RECEIPT
        );
        assert!(
            state
                .proves_occurrence_receipt(&self.event, &self.identity)
                .unwrap()
        );
        assert_eq!(
            state.acknowledged_events_for_test().back(),
            Some(&self.event)
        );
        self.assert_decision();
    }

    pub(in crate::worker::codex_hook::tests) fn assert_raw_removed_receipt_retained(&self) {
        assert!(
            self.intake
                .find_exact_occurrence(&self.occurrence)
                .unwrap()
                .is_none()
        );
        let bytes = std::fs::read(self.outbox.path()).unwrap();
        let state: crate::codex_hooks::CodexOutboxState = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            state.occurrence_receipt_count_for_test(),
            SINGLE_OCCURRENCE_RECEIPT
        );
        assert!(
            state
                .proves_occurrence_receipt(&self.event, &self.identity)
                .unwrap()
        );
    }

    pub(in crate::worker::codex_hook::tests) fn assert_cleanup_complete(&self) {
        assert!(
            self.intake
                .find_exact_occurrence(&self.occurrence)
                .unwrap()
                .is_none()
        );
        let state = self.outbox.load().unwrap();
        assert!(state.occurrence_receipt_count_for_test() == usize::default());
        assert_eq!(
            state.acknowledged_events_for_test().back(),
            Some(&self.event)
        );
        self.assert_decision();
    }
}
