use super::ControlChannel;
use crate::relay::constants::{CONTROL_CONNECTION_INVARIANT, CONTROL_RECONNECT_DELAY};
use crate::relay::{RelayError, client_error_is_retryable, relay_error_is_retryable};
use serde::Serialize;
use serde::de::DeserializeOwned;

impl ControlChannel {
    pub(super) async fn request_replay_safe<T, R>(
        &mut self,
        method_name: &str,
        payload: &T,
    ) -> Result<R, RelayError>
    where
        T: Serialize,
        R: DeserializeOwned,
    {
        loop {
            let result = self
                .connection
                .as_mut()
                .expect(CONTROL_CONNECTION_INVARIANT)
                .request(method_name, payload)
                .await;
            match result {
                Ok(value) => return Ok(value),
                Err(error) if client_error_is_retryable(&error) => {
                    self.connection = None;
                    self.reconnect_replay_safe().await?;
                }
                Err(error) => return Err(error.into()),
            }
        }
    }

    async fn reconnect_replay_safe(&mut self) -> Result<(), RelayError> {
        loop {
            tokio::time::sleep(CONTROL_RECONNECT_DELAY).await;
            match self.runtime.reconnect(self.target.terminal_id).await {
                Ok(()) => {}
                Err(error) if relay_error_is_retryable(&error) => continue,
                Err(error) => return Err(error),
            }
            match self.runtime.client().connect().await {
                Ok(connection) => {
                    self.connection = Some(connection);
                    return Ok(());
                }
                Err(error) if client_error_is_retryable(&error) => {}
                Err(error) => return Err(error.into()),
            }
        }
    }
}
