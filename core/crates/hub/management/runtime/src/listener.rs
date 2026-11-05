use anyhow::{Context, Result};
use std::time::Duration;
use yolo_hub_state::HubState;
use yolo_paths::socket_activation::{self, SocketPublication};

const PROBE_TIMEOUT: Duration = Duration::from_secs(1);
const LIVE: &str = "another Hub owns this peer socket";
const BIND: &str = "Hub peer socket could not be published";

/// The Hub's only peer entry point: a private Unix socket in Hub state.
/// The outer SSH login decides who can reach it; peer policy still decides
/// which Hub identities may use it.
pub(crate) struct PeerListener {
    listener: std::os::unix::net::UnixListener,
    publication: Option<SocketPublication>,
}

impl PeerListener {
    pub(crate) async fn publish(state: &HubState) -> Result<Self> {
        let path = state.peer_socket()?;
        let (listener, publication) =
            socket_activation::publish(state.directory()?, &path, PROBE_TIMEOUT, LIVE).await?;
        let listener = listener
            .into_std()
            .context(BIND)?;
        listener.set_nonblocking(true).context(BIND)?;
        Ok(Self {
            listener,
            publication: Some(publication),
        })
    }

    pub(crate) fn activate(&self) -> Result<tokio::net::UnixListener> {
        Ok(tokio::net::UnixListener::from_std(
            self.listener.try_clone().context(BIND)?,
        )?)
    }
}

impl Drop for PeerListener {
    fn drop(&mut self) {
        if let Some(publication) = self.publication.take() {
            let _ = publication.withdraw();
        }
    }
}
