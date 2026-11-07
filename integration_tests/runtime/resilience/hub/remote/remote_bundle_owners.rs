use super::constants::*;
use crate::ProcessIdentity;
use std::path::{Path, PathBuf};
use yolo_hub_client::Client;
use yolo_hub_state::HubState;

pub struct Owners {
    pub host: Option<ProcessIdentity>,
    pub hub: Option<ProcessIdentity>,
    pub home: PathBuf,
    pub state: PathBuf,
    owns_host: bool,
}
impl Owners {
    pub fn new(home: &Path, state: &Path, owns_host: bool) -> Self {
        Self {
            host: None,
            hub: None,
            home: home.into(),
            state: state.into(),
            owns_host,
        }
    }
    /// Capture only independently authenticated processes in this fixture's private namespace.
    pub async fn observe(&mut self) {
        let _ = tokio::time::timeout(PROBE_TIMEOUT, self.observe_inner()).await;
    }
    async fn observe_inner(&mut self) -> anyhow::Result<()> {
        if self.host.is_none() {
            let paths = yolo_runtime::paths::Paths::new(&self.home);
            let client = yolo_client::Client::managed_local(&self.home, paths.host_socket()?)?;
            let mut connection = client.connect().await?;
            let status: yolo_runtime::model::HostStatus = connection
                .request(yolo_runtime::protocol::method::HOST_STATUS, &())
                .await?;
            let identity = ProcessIdentity::capture(status.host_pid)
                .ok_or_else(|| anyhow::anyhow!(HOST_CUSTODY_LOST))?;
            let fresh: yolo_runtime::model::HostStatus = connection
                .request(yolo_runtime::protocol::method::HOST_STATUS, &())
                .await?;
            let credentials = connection.peer_credentials();
            anyhow::ensure!(
                u32::try_from(credentials.pid().unwrap())? == identity.pid
                    && fresh.host_pid == identity.pid
                    && identity.matches()
            );
            self.host = Some(identity);
        }
        if self.hub.is_none() {
            let peer = HubState::inspect_peer_identity(&self.state, &self.home)?
                .ok_or_else(|| anyhow::anyhow!(HUB_IDENTITY_PENDING))?;
            let socket = HubState::socket_path_for(&self.state)?;
            let client = Client::connect_local(&socket, peer.hub(), deadline()).await?;
            let stream = client.into_channel()?.into_stream()?;
            let pid = u32::try_from(stream.get_ref().peer_cred()?.pid().unwrap())?;
            let identity =
                ProcessIdentity::capture(pid).ok_or_else(|| anyhow::anyhow!(HUB_CUSTODY_LOST))?;
            Client::open(stream, peer.hub(), deadline()).await?;
            anyhow::ensure!(identity.matches());
            self.hub = Some(identity);
        }
        Ok(())
    }
    pub fn identities(&self) -> (ProcessIdentity, ProcessIdentity) {
        (self.host.unwrap(), self.hub.unwrap())
    }
    pub fn complete(&self) -> bool {
        self.host.is_some() && self.hub.is_some()
    }
    pub async fn stop(&mut self) {
        if let Some(hub) = self.hub {
            hub.signal(libc::SIGTERM).unwrap();
            crate::wait_for_identity_gone(hub).await;
            self.hub = None;
        }
        if self.owns_host {
            if let Some(host) = self.host {
                host.signal(libc::SIGINT).unwrap();
                crate::wait_for_identity_gone(host).await;
                self.host = None;
            }
        }
    }
}
impl Drop for Owners {
    fn drop(&mut self) {
        if let Some(hub) = self.hub {
            retire(hub, libc::SIGTERM);
        }
        if self.owns_host {
            if let Some(host) = self.host {
                retire(host, libc::SIGINT);
            }
        }
    }
}

fn retire(identity: ProcessIdentity, signal: i32) {
    let _ = identity.signal(signal);
    if wait_gone(identity) {
        return;
    }
    if identity.signal(libc::SIGKILL).unwrap_or(false) {
        crate::discard_incomplete_coverage_profile(identity.pid);
    }
    if !wait_gone(identity) {
        tracing::error!(?identity, "remote bundle fixture process did not retire");
    }
}
fn wait_gone(identity: ProcessIdentity) -> bool {
    let end = std::time::Instant::now() + CLEANUP_TIMEOUT;
    loop {
        if identity.observe() == yolo_process_test_support::ProcessIdentityObservation::Gone {
            return true;
        }
        if std::time::Instant::now() >= end {
            return false;
        }
        std::thread::sleep(POLL);
    }
}
