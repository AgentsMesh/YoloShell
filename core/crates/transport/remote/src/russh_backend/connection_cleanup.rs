impl RusshConnection {
    pub async fn disconnect(&self) -> Result<(), RemoteError> {
        let cleanup = self
            .cleanup
            .settle(cleanup_settlement_timeout(self.operation_timeout))
            .await;
        let disconnected = self.disconnect_transport().await;
        settle_disconnect(cleanup, disconnected)
    }
}

impl RusshConnection {
    async fn disconnect_transport(&self) -> Result<(), RemoteError> {
        channel_open_disconnect::disconnect_session(&self.session, self.operation_timeout).await
    }
}

fn settle_disconnect(
    cleanup: Result<(), RemoteError>,
    disconnected: Result<(), RemoteError>,
) -> Result<(), RemoteError> {
    match (cleanup, disconnected) {
        (Ok(()), result) => result,
        (Err(primary), Ok(())) => Err(primary),
        (Err(primary), Err(settlement)) => Err(RemoteError::LifecycleSettlement {
            primary: Box::new(primary),
            settlement: Box::new(settlement),
        }),
    }
}
