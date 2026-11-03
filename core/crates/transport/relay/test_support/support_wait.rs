use super::FakeHost;
use super::values::{METHOD_WAIT_TIMEOUT, METHOD_WAIT_TIMEOUT_DETAIL, METHOD_WATCH_CLOSED_DETAIL};

impl FakeHost {
    pub(crate) async fn wait_for_method(&self, expected: &str) {
        let mut arrived = self.request_arrived.subscribe();
        let observed = tokio::time::timeout(METHOD_WAIT_TIMEOUT, async {
            loop {
                if self.methods().iter().any(|method| method == expected) {
                    return;
                }
                arrived.changed().await.expect(METHOD_WATCH_CLOSED_DETAIL);
            }
        })
        .await;
        assert!(
            observed.is_ok(),
            "{METHOD_WAIT_TIMEOUT_DETAIL}: expected={expected}; methods={:?}",
            self.methods(),
        );
    }
}
