use super::constants::*;
use crate::hub::{
    relay_crash::process::RelayProcess,
    remote_bundle::{assets, owners::Owners},
    ssh_profile_live::{broker::Broker, observations},
};
use std::path::PathBuf;
use yolo_broker_hub_protocol::BrokerHubProfile;
use yolo_distribution_bundle::BundleSource;
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_remote_setup_protocol::*;
use yolo_hub_remote_startup_protocol::*;
use yolo_hub_routing::Right;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_ssh_profile_protocol::SshProfileConfiguration;
use yolo_hub_state::HubState;
use yolo_remote_provisioning_protocol::*;

pub struct Fixture {
    pub owners: Owners,
    pub broker: Broker,
    pub entry_peer: PeerIdentity,
    pub entry_state: PathBuf,
    pub entry_socket: PathBuf,
    pub install: PathBuf,
    pub home: PathBuf,
    pub remote_state: PathBuf,
    pub source: BundleSource,
    pub broker_captured: bool,
    _root: tempfile::TempDir,
}
impl Fixture {
    pub async fn new(entry: &crate::Harness, sshd: &crate::ssh_transport::SshdFixture) -> Self {
        let root = tempfile::tempdir().unwrap();
        let base = root.path().canonicalize().unwrap();
        let home = base.join(REMOTE_HOME);
        crate::prepare_host_home(&home);
        let remote_state = base.join(REMOTE_STATE);
        let entry_state = base.join(ENTRY_STATE);
        let state = HubState::open(&entry_state, entry.home.path()).unwrap();
        let entry_peer = state.peer_identity();
        let entry_socket = state.socket_path();
        drop(state);
        let source = assets::upload_source(&base.join(SOURCE_DIRECTORY)).await;
        let broker = Broker::prepare(&base, &home, sshd);
        Self {
            owners: Owners::new(&home, &remote_state, true),
            broker,
            entry_peer,
            entry_state,
            entry_socket,
            install: base.join(REMOTE_INSTALL),
            home,
            remote_state,
            source,
            broker_captured: false,
            _root: root,
        }
    }
    pub async fn register(&self, entry: &RelayProcess) {
        assert!(
            observations::profiles(entry, self.entry_peer.hub())
                .await
                .configuration
                .profiles
                .is_empty()
        );
        observations::configure(
            entry,
            self.entry_peer.hub(),
            SshProfileConfiguration {
                profiles: vec![self.broker.registration.clone()],
                ..Default::default()
            },
        )
        .await;
    }
    pub async fn request(&self, entry: &RelayProcess) -> RemoteSetupRequest {
        let Reply::Configuration { snapshot } = entry
            .client()
            .await
            .call(
                self.entry_peer.hub(),
                Command::GetConfiguration {},
                deadline(),
            )
            .await
            .unwrap()
        else {
            panic!()
        };
        let profile = &self.broker.registration;
        let id = uuid::Uuid::new_v4();
        let bootstrap = RemoteBootstrapRequest::new(RemoteBootstrapRequestFields {
            schema_version: REMOTE_BOOTSTRAP_SCHEMA_VERSION,
            request_id: id,
            bundle: self.source.descriptor().clone(),
            expected_peer: None,
            consumer: self.entry_peer,
            rights: [
                Right::Observe,
                Right::TerminalRead,
                Right::TerminalWrite,
                Right::Lifecycle,
            ]
            .into_iter()
            .collect(),
            remaining_reexports: REEXPORTS,
        })
        .unwrap();
        RemoteSetupRequest {
            expected_configuration_revision: snapshot.state.revision,
            request: ProvisionRequest::new(ProvisionFields {
                schema_version: PROVISION_SCHEMA_VERSION,
                request_id: id,
                profile: BrokerHubProfile {
                    host_id: profile.profile.profile_id().into(),
                    profile_authority_sha256: profile.profile.authority_sha256().into(),
                    config_revision: profile.config_revision.clone(),
                },
                source: self.source.clone(),
                install_root: self.install.clone(),
                state: self.remote_state.clone(),
                host_home: self.home.clone(),
                bootstrap,
            })
            .unwrap(),
        }
    }
    pub fn direct_host(&self) -> yolo_client::Client {
        yolo_client::Client::managed_local(
            &self.home,
            crate::Paths::new(&self.home).host_socket().unwrap(),
        )
        .unwrap()
    }
    pub async fn finish(self) {
        let Self {
            mut owners,
            broker,
            _root,
            ..
        } = self;
        owners.stop().await;
        broker.stop().await;
        drop(_root);
    }
}
#[path = "poll.rs"]
mod poll;
