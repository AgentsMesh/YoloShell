use super::{
    super::federation_support::{POLL, deadline},
    constants::*,
};
use std::{
    io::Read,
    os::unix::{fs::OpenOptionsExt, process::ExitStatusExt},
    path::{Path, PathBuf},
};
use yolo_hub_client::{Client, LocalClient};
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_federation_settings::Configuration;
use yolo_runtime::process::ScopedWaitableChild;
#[path = "relay_crash_rejection.rs"]
mod rejection;
#[path = "relay_crash_spawn.rs"]
mod spawn;

pub struct RelayProcess {
    child: ScopedWaitableChild,
    identity: crate::ProcessIdentity,
    peer: PeerIdentity,
    socket: PathBuf,
    _log: tempfile::NamedTempFile,
}

impl RelayProcess {
    pub async fn start(
        harness: &crate::Harness,
        state: &Path,
        socket: &Path,
        peer: PeerIdentity,
        configuration: &Path,
    ) -> Self {
        Self::start_configured(harness, state, socket, peer, Some(configuration)).await
    }

    pub async fn start_existing(
        harness: &crate::Harness,
        state: &Path,
        socket: &Path,
        peer: PeerIdentity,
    ) -> Self {
        Self::start_configured(harness, state, socket, peer, None).await
    }

    async fn start_configured(
        harness: &crate::Harness,
        state: &Path,
        socket: &Path,
        peer: PeerIdentity,
        configuration: Option<&Path>,
    ) -> Self {
        let mut relay = Self::spawn_configured(harness, state, socket, peer, configuration);
        let identity = relay.identity;
        let ready = tokio::time::timeout_at(deadline(), async {
            loop {
                if relay.child.try_wait().unwrap().is_some() {
                    return false;
                }
                if let Ok(client) = Client::connect_local(socket, peer.hub(), deadline()).await {
                    let stream = client.into_channel().unwrap().into_stream().unwrap();
                    let pid = u32::try_from(stream.get_ref().peer_cred().unwrap().pid().unwrap()).unwrap();
                    assert_eq!(pid, identity.pid);
                    Client::open(stream, peer.hub(), deadline()).await.unwrap();
                    assert!(identity.matches());
                    return true;
                }
                tokio::time::sleep(POLL).await;
            }
        })
        .await;
        assert!(
            matches!(ready, Ok(true)),
            "{RELAY_START_FAILURE}: {}",
            relay.diagnostic()
        );
        relay
    }

    pub fn diagnostic(&self) -> String {
        let mut bytes = Vec::new();
        self._log
            .reopen()
            .unwrap()
            .take(MAX_DIAGNOSTIC_BYTES)
            .read_to_end(&mut bytes)
            .unwrap();
        String::from_utf8_lossy(&bytes).into_owned()
    }

    pub async fn client(&self) -> LocalClient {
        Client::connect_local(&self.socket, self.peer.hub(), deadline())
            .await
            .unwrap()
    }

    pub async fn crash(self) -> crate::ProcessIdentity {
        let identity = self.identity;
        let status = self
            .child
            .terminate_and_wait_status(deadline().into_std())
            .await
            .unwrap();
        assert_eq!(status.signal(), Some(libc::SIGKILL));
        crate::discard_incomplete_coverage_profile(identity.pid);
        assert!(!identity.matches());
        identity
    }

    pub async fn stop(self) {
        assert!(self.identity.signal(libc::SIGTERM).unwrap());
        let log = self._log.path().to_path_buf();
        if let Err(error) = self.child.wait_for_clean_exit(deadline().into_std()).await {
            let detail = std::fs::read_to_string(log)
                .unwrap_or_else(|read_error| format!("relay log unavailable: {read_error}"));
            panic!("{error}; relay stderr: {detail}");
        }
    }
}

pub fn write_configuration(directory: &Path, configuration: &Configuration) -> PathBuf {
    let path = directory.join(CONFIGURATION_FILE);
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(PRIVATE_FILE_MODE)
        .open(&path)
        .unwrap();
    serde_json::to_writer(&file, configuration).unwrap();
    file.sync_all().unwrap();
    path
}
