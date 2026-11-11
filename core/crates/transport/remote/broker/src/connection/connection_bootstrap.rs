use super::BrokerConnectionPool;
use super::connection::ObservedConnection;
use super::connection_runtime::{
    channel_open_timed_out, connection_lost, next_delay, runtime_probe_retryable,
};
use super::constants::INITIAL_RECONNECT_DELAY;
use super::protocol::BrokerStatus;
use super::timing::CONNECTION_BOOTSTRAP_TIMEOUT;
use crate::remote::RemoteError;

pub(super) const CONNECTION_BOOTSTRAP_TIMEOUT_MESSAGE: &str =
    "remote connection bootstrap or runtime deployment timed out";

impl BrokerConnectionPool {
    pub async fn bootstrap(&self) -> Result<BrokerStatus, RemoteError> {
        self.require_runtime_mode()?;
        tokio::time::timeout(CONNECTION_BOOTSTRAP_TIMEOUT, self.bootstrap_until_live())
            .await
            .map_err(|_| {
                RemoteError::RuntimeDeployment(CONNECTION_BOOTSTRAP_TIMEOUT_MESSAGE.into())
            })?
    }

    pub(super) async fn bootstrap_connection(&self) -> Result<ObservedConnection, RemoteError> {
        self.require_runtime_mode()?;
        tokio::time::timeout(CONNECTION_BOOTSTRAP_TIMEOUT, self.connect_until_available())
            .await
            .map_err(|_| {
                RemoteError::RuntimeDeployment(CONNECTION_BOOTSTRAP_TIMEOUT_MESSAGE.into())
            })?
    }

    async fn connect_until_available(&self) -> Result<ObservedConnection, RemoteError> {
        let mut delay = INITIAL_RECONNECT_DELAY;
        loop {
            match self.observed_connection().await {
                Ok(connection) => return Ok(connection),
                Err(error) if connection_lost(&error) => {
                    tokio::time::sleep(delay).await;
                    delay = next_delay(delay);
                }
                Err(error) => return Err(error),
            }
        }
    }

    async fn bootstrap_until_live(&self) -> Result<BrokerStatus, RemoteError> {
        let mut delay = INITIAL_RECONNECT_DELAY;
        loop {
            let observed = self.connect_until_available().await?;
            match self.probe_connection(&observed).await {
                Ok(()) if self.is_retired(&observed) => {
                    self.invalidate(&observed).await;
                }
                Ok(()) => return Ok(self.status_for(&observed)),
                Err(error) if channel_open_timed_out(&error) => {
                    self.invalidate(&observed).await;
                    return Err(error);
                }
                Err(error) if runtime_probe_retryable(&error) => {
                    self.invalidate(&observed).await;
                }
                Err(error) => return Err(error),
            }
            tokio::time::sleep(delay).await;
            delay = next_delay(delay);
        }
    }
}
