use super::constants::*;
use crate::hub::{
    federation_support::{RunningNode, TEMP_ROOT, exports, listener, peer},
    local_startup_support::retain_fixture_host,
    relay_crash::process::{self, RelayProcess},
};
use std::{path::PathBuf, sync::Arc};
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_federation_settings::Configuration;
use yolo_hub_state::HubState;

pub struct Location {
    pub peer: PeerIdentity,
    pub state: PathBuf,
    pub socket: PathBuf,
    pub configuration: PathBuf,
}
impl Location {
    fn new(harness: &crate::Harness, state: PathBuf, configuration: &Configuration) -> Self {
        let owner = HubState::open(&state, harness.home.path()).unwrap();
        let peer = owner.peer_identity();
        let socket = owner.socket_path();
        drop(owner);
        let configuration = process::write_configuration(&state, configuration);
        Self {
            peer,
            state,
            socket,
            configuration,
        }
    }
    pub async fn start(&self, host: &crate::Harness) -> RelayProcess {
        RelayProcess::start(
            host,
            &self.state,
            &self.socket,
            self.peer,
            &self.configuration,
        )
        .await
    }
    pub async fn reopen(&self, host: &crate::Harness) -> RelayProcess {
        RelayProcess::start_existing(host, &self.state, &self.socket, self.peer).await
    }
}

pub struct Topology {
    pub entry_host: crate::agent_recovery::FederationHost,
    pub relay_host: crate::agent_recovery::FederationHost,
    pub origin_host: crate::agent_recovery::FederationHost,
    pub entry: Location,
    pub relay: Location,
    pub origin: RunningNode,
    _holds: Vec<yolo_client::HostHold>,
    _root: tempfile::TempDir,
}
impl Topology {
    pub async fn new() -> Self {
        let entry_host = crate::agent_recovery::federation_harness().await;
        let relay_host = crate::agent_recovery::federation_harness().await;
        let origin_host = crate::agent_recovery::federation_harness().await;
        let holds = vec![
            retain_fixture_host(&entry_host).await,
            retain_fixture_host(&relay_host).await,
            retain_fixture_host(&origin_host).await,
        ];
        let root = tempfile::tempdir_in(TEMP_ROOT).unwrap();
        let path = root.path().canonicalize().unwrap();
        let entry_path = path.join(ENTRY_DIRECTORY);
        let relay_path = path.join(RELAY_DIRECTORY);
        let entry = HubState::open(&entry_path, entry_host.home.path())
            .unwrap()
            .peer_identity();
        let relay = HubState::open(&relay_path, relay_host.home.path())
            .unwrap()
            .peer_identity();
        let origin_state = Arc::new(
            HubState::open(&path.join(ORIGIN_DIRECTORY), origin_host.home.path()).unwrap(),
        );
        let origin = origin_state.peer_identity();
        let relay_listener = listener().await;
        let relay_address = relay_listener.local_addr().unwrap();
        let origin_listener = listener().await;
        let origin_config = Configuration {
            listen: Some(origin_listener.local_addr().unwrap()),
            peers: vec![peer(
                relay,
                Some(relay_address),
                false,
                vec![exports(origin.hub(), true)],
            )],
            ..Configuration::default()
        };
        let entry_config = Configuration {
            trusted_origins: vec![origin],
            peers: vec![peer(relay, Some(relay_address), true, vec![])],
            ..Configuration::default()
        };
        let relay_config = Configuration {
            listen: Some(relay_address),
            peers: vec![
                peer(
                    entry,
                    None,
                    false,
                    vec![exports(relay.hub(), false), exports(origin.hub(), false)],
                ),
                peer(origin, None, true, vec![]),
            ],
            ..Configuration::default()
        };
        let entry_location = Location::new(&entry_host, entry_path, &entry_config);
        let relay_location = Location::new(&relay_host, relay_path, &relay_config);
        assert_eq!(entry_location.peer, entry);
        assert_eq!(relay_location.peer, relay);
        drop(relay_listener);
        let origin =
            RunningNode::start(&origin_host, origin_state, origin_config, origin_listener).await;
        Self {
            entry_host,
            relay_host,
            origin_host,
            entry: entry_location,
            relay: relay_location,
            origin,
            _holds: holds,
            _root: root,
        }
    }
    pub async fn finish(self) {
        self.origin.stop().await;
        drop(self._holds);
        drop((self.entry_host, self.relay_host, self.origin_host));
        drop(self._root);
    }
}
