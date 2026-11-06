mod bootstrap_document;
mod constants;
mod document;
mod fence;
mod host;
mod hub;
mod namespaces;
mod retry;

use anyhow::Result;
pub use bootstrap_document::BootstrapDocument;
pub use constants::{
    BOOTSTRAP_RETIRE_TIMEOUT, CONFIG_SCHEMA_VERSION, FILE_HELPER_ARGUMENT, HOST_SOCKET_ARGUMENT,
    MAX_READY_BYTES, PEER_CONFIG_ARGUMENT, READY_ARGUMENT, STARTUP_TIMEOUT,
};
pub(crate) use yolo_launch_contract::{HOST_HOME_ARGUMENT, HUB_STATE_ARGUMENT};
pub use document::{ConnectionDocument, ReadyDocument};
use std::path::PathBuf;
use tokio::time::Instant;
use yolo_hub_delegation::PeerIdentity;

#[derive(Clone, Debug)]
pub struct Launch {
    pub state: PathBuf,
    pub host_home: PathBuf,
    pub host_executable: PathBuf,
    pub hub_executable: PathBuf,
}

pub async fn ensure_running(launch: Launch, deadline: Instant) -> Result<ConnectionDocument> {
    ensure_running_with_peer(launch, None, deadline).await
}

/// Proves this process can launch a new Host. Reusing an already running Host
/// does not need custody, but a restart stops that Host first, so callers that
/// stop services must pass this gate before mutating anything.
pub fn require_host_custody() -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        host::require_custody()
    }
    #[cfg(not(target_os = "linux"))]
    {
        Ok(())
    }
}

pub async fn ensure_running_with_peer(
    launch: Launch,
    expected_peer: Option<PeerIdentity>,
    deadline: Instant,
) -> Result<ConnectionDocument> {
    let fence = fence::StartupFence::acquire(&launch, expected_peer, deadline).await?;
    finish(launch, fence, deadline).await
}

/// Keeps a caller's already retained state directory identity across startup lock waits.
pub async fn ensure_running_with_peer_in(
    launch: Launch,
    expected_peer: Option<PeerIdentity>,
    parent: yolo_paths::directory::PrivateDirectory,
    deadline: Instant,
) -> Result<ConnectionDocument> {
    let fence = fence::StartupFence::acquire_in(&launch, expected_peer, parent, deadline).await?;
    finish(launch, fence, deadline).await
}

async fn finish(
    launch: Launch,
    fence: fence::StartupFence,
    deadline: Instant,
) -> Result<ConnectionDocument> {
    retry::require_budget(deadline)?;
    let namespaces = namespaces::StartupNamespaces::retain(&launch, &fence)?;
    let identity = fence.peer.hub();
    let socket = namespaces.hub_socket()?;
    let host_socket = namespaces.host_socket()?;
    let _host = host::ensure(&launch, &host_socket, &fence, deadline)
        .await?
        .commit(&fence, deadline)
        .await?;
    fence.verify()?;
    namespaces.verify()?;
    hub::ensure(&launch, &socket, &host_socket, &fence, deadline).await?;
    fence.verify()?;
    namespaces.verify()?;
    retry::require_budget(deadline)?;
    Ok(ConnectionDocument::local(identity, socket))
}

#[cfg(all(test, not(feature = "yolo-integration-tests")))]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/lib.rs"]
mod integration_tests;
mod readiness;
pub use readiness::encode_ready;
