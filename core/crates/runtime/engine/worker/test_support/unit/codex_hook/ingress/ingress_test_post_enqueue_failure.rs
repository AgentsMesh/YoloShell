use super::PendingIngressExchange;
use crate::agent::hooks::HookEnvelope;
use crate::worker::codex_hook::protocol::CodexHookAck;

impl PendingIngressExchange {
    pub(in crate::worker::codex_hook::tests) async fn finish_post_enqueue_failure(
        self,
    ) -> CodexHookAck {
        self.finish_post_enqueue_failure_detailed().await.0
    }

    pub(in crate::worker::codex_hook::tests) async fn finish_post_enqueue_failure_detailed(
        self,
    ) -> (CodexHookAck, HookEnvelope) {
        let occurrence = self.occurrence.clone();
        let ack = self.client.finish().await.unwrap();
        (ack, occurrence)
    }
}
