impl BrokerConnectionPool {
    pub(super) async fn settle_cleanup(&self) -> Result<(), RemoteError> {
        self.cleanup
            .settle(super::constants::REMOTE_CLEANUP_DRAIN_TIMEOUT)
            .await
    }
}
