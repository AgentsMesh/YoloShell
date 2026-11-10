use crate::{LocalBackend, errors::map_error};
use yolo_hub_terminal_protocol::PublicError;

impl LocalBackend {
    pub async fn hold_host(&self, stop: tokio::sync::watch::Receiver<bool>) {
        self.client.hold_host(stop).await;
    }

    pub async fn retain_host(&self) -> Result<yolo_client::HostHold, PublicError> {
        self.client.retain_host().await.map_err(map_error)
    }
}
