use super::*;

impl Client {
    pub async fn shutdown_if_idle(&self) -> Result<(), ClientError> {
        let deadline = self.operation_deadline();
        let mut connection = self.connect_until(deadline).await?;
        let status: HostStatus = match connection.take_authenticated_status() {
            Some(status) => status,
            None => {
                connection
                    .request_until(method::HOST_STATUS, &(), deadline)
                    .await?
            }
        };
        let capability = crate::protocol::capability::IDLE_HOST_SHUTDOWN;
        if !status.capabilities.iter().any(|value| value == capability) {
            return Err(ClientError::HostCapabilityUnavailable {
                capability: capability.to_owned(),
            });
        }
        connection
            .request_until(method::SHUTDOWN_IF_IDLE, &(), deadline)
            .await
    }
}
