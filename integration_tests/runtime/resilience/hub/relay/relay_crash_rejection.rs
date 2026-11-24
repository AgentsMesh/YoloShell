use super::*;

impl RelayProcess {
    pub async fn expect_rejected(
        harness: &crate::Harness,
        state: &Path,
        socket: &Path,
        peer: PeerIdentity,
    ) -> String {
        let mut relay = Self::spawn_configured(harness, state, socket, peer, None);
        let status = tokio::time::timeout_at(deadline(), async {
            loop {
                if let Some(status) = relay.child.try_wait().unwrap() {
                    return status;
                }
                assert!(
                    Client::connect_local(socket, peer.hub(), deadline())
                        .await
                        .is_err()
                );
                tokio::time::sleep(POLL).await;
            }
        })
        .await
        .unwrap();
        let diagnostic = relay.diagnostic();
        assert!(
            !status.success(),
            "unexpectedly accepted invalid listener: {diagnostic}"
        );
        assert!(!relay.identity.matches());
        diagnostic
    }
}
