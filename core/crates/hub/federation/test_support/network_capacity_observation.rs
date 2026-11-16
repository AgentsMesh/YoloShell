use super::{constants::*, fixture::LinkFixture};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    time::{Instant, timeout},
};
use yolo_hub_peer_protocol::FIRST_REVISION;
use yolo_hub_peer_protocol_test_support::support::observe;

impl LinkFixture {
    pub async fn assert_retained(&mut self) {
        assert_eq!(
            self.node
                .current(self.link.connection.peer().hub())
                .await
                .unwrap()
                .id,
            self.link.id
        );
        assert!(!self.link.connection.is_closed());
        self.assert_route(crate::node::now_ms().unwrap(), true)
            .await;
        self.assert_route(self.responder.export.catalogue.payload.expires_at_ms, false)
            .await;
        timeout(TEST_TIMEOUT, async {
            self.output.write_all(STREAM_BYTES).await.unwrap();
            let mut bytes = vec![u8::default(); STREAM_BYTES.len()];
            self.input.read_exact(&mut bytes).await.unwrap();
            assert_eq!(bytes, STREAM_BYTES);
            self.input.write_all(STREAM_BYTES).await.unwrap();
            self.output.read_exact(&mut bytes).await.unwrap();
            assert_eq!(bytes, STREAM_BYTES);
        })
        .await
        .unwrap();
    }

    async fn assert_route(&self, now: u64, present: bool) {
        assert_eq!(
            self.node
                .book
                .lock()
                .await
                .route_for(self.link.connection.peer().hub(), &observe(), now,)
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
                    .latest_for(self.link.connection.peer().hub())
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
        self.peer
            .close(Instant::now() + TEST_TIMEOUT)
            .await
            .unwrap();
        self.node.begin_shutdown();
        self.node
            .observation_cache
            .wait_closed(Instant::now() + TEST_TIMEOUT)
            .await;
    }
}
