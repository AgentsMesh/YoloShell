impl BrokerClient {
    pub async fn bootstrap(&self) -> Result<BrokerStatus, BrokerClientError> {
        self.bootstrap_inner(None, operation_deadline()).await
    }

    pub(super) async fn bootstrap_at(
        &self,
        expected: SocketIdentity,
        deadline: tokio::time::Instant,
    ) -> Result<BrokerStatus, BrokerClientError> {
        self.bootstrap_inner(Some(expected), deadline).await
    }

    async fn bootstrap_inner(
        &self,
        expected: Option<SocketIdentity>,
        deadline: tokio::time::Instant,
    ) -> Result<BrokerStatus, BrokerClientError> {
        let mut connection = self.connect(expected, deadline).await?;
        let status = bootstrap_on(connection.stream(), deadline).await?;
        connection.verify_path()?;
        Ok(status)
    }
}
