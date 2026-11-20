use super::{constants::*, fixture::LinkFixture};
use tokio::time::{Instant, timeout};
use yolo_hub_peer_protocol::FIRST_REVISION;
use yolo_hub_peer_protocol_test_support::support::observe;

impl LinkFixture {
    pub async fn assert_retained(&self) {
        assert_eq!(
            self.node
                .current(self.link.peer.hub())
                .await
                .unwrap()
                .id,
            self.link.id
        );
        assert!(!self.link.is_closed());
        self.assert_route(crate::node::now_ms().unwrap(), true)
            .await;
        self.assert_route(self.responder.export.catalogue.payload.expires_at_ms, false)
            .await;
    }

    async fn assert_route(&self, now: u64, present: bool) {
        assert_eq!(
            self.node
                .book
                .lock()
                .await
                .route_for(self.link.peer.hub(), &observe(), now,)
                .unwrap()
                .is_some(),
            present
        );
    }

    pub async fn wait_refreshed(&self) {
        timeout(TEST_TIMEOUT, async {
            loop {
                let revised = self
                    .node
                    .book
                    .lock()
                    .await
                    .latest_for(self.link.peer.hub())
                    .is_some_and(|source| source.payload.revision > FIRST_REVISION);
                if revised {
                    break;
                }
                tokio::time::sleep(PROGRESS_POLL).await;
            }
        })
        .await
        .unwrap();
    }

    pub async fn close(&self) {
        self.server.abort();
        self.node.begin_shutdown();
        self.node
            .observation_cache
            .wait_closed(Instant::now() + TEST_TIMEOUT)
            .await;
    }
}
