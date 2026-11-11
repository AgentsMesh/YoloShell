use super::super::super::ingress_live_client::PendingPeerExchange;
use super::super::super::ingress_live_source::PeerEvidence;
use super::{IngressEndpoint, PendingIngressExchange};
use crate::agent::hooks::HookEnvelope;

impl PendingIngressExchange {
    pub(in crate::worker::codex_hook::tests) async fn finish_indeterminate(self) -> HookEnvelope {
        let occurrence = self.occurrence.clone();
        assert!(self.client.finish().await.is_err());
        occurrence
    }
}

impl IngressEndpoint {
    pub(in crate::worker::codex_hook::tests) async fn retry_without_ack(
        &self,
        server: &super::super::super::super::server::CodexHookServer,
        occurrence: &HookEnvelope,
    ) {
        let client = PendingPeerExchange::begin(
            self.assets.live_source(),
            self.assets.hook_executable(),
            server.socket_path(),
            occurrence,
            PeerEvidence::Direct,
        );
        let stream = server.accept().await.unwrap().0;
        let (handled, acknowledgement) =
            tokio::join!(server.handle_stream(stream), client.finish());
        let (_, response) = handled.unwrap().into_parts();
        assert!(response.is_err());
        assert!(acknowledgement.is_err());
    }
}
