use super::RelayRuntime;
use crate::client::Client;
use crate::relay::constants::{BROKER_RECONNECT_INITIAL_INTERVAL, BROKER_RECONNECT_MAX_INTERVAL};
use crate::relay::{RelayError, host::observation_interrupted};
use std::path::{Path, PathBuf};
use yolo_transport_core::AcceptBackoff;

impl RelayRuntime {
    pub fn broker(client: Client) -> Result<Self, RelayError> {
        crate::remote_broker::validate_broker_socket_path(client.socket_path())
            .map_err(unsafe_broker_socket)?;
        Ok(Self {
            client,
            launch: None,
            wait_for_broker: true,
        })
    }

    pub(super) async fn wait_for_broker(&self) -> Result<Option<PathBuf>, RelayError> {
        let mut backoff =
            AcceptBackoff::new(BROKER_RECONNECT_INITIAL_INTERVAL, BROKER_RECONNECT_MAX_INTERVAL);
        loop {
            validate_waiting_socket(self.client.socket_path())?;
            match self.client.status().await {
                Ok(_) => return Ok(None),
                Err(error) if observation_interrupted(&error) => {
                    tokio::time::sleep(backoff.next_delay()).await;
                }
                Err(error) => return Err(error.into()),
            }
        }
    }
}

fn validate_waiting_socket(path: &Path) -> Result<(), RelayError> {
    match std::fs::symlink_metadata(path) {
        Ok(_) => crate::remote_broker::validate_broker_socket(path).map_err(unsafe_broker_socket),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn unsafe_broker_socket(error: anyhow::Error) -> RelayError {
    std::io::Error::new(std::io::ErrorKind::PermissionDenied, error.to_string()).into()
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/host/broker_tests.rs"]
mod tests;
