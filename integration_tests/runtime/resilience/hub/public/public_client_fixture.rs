use super::{constants::*, credentials, deadline};
use crate::{Harness, hub::relay_crash::process::RelayProcess, ssh_transport::SshdFixture};
use russh::keys::PrivateKey;
use std::{path::PathBuf, sync::Arc};
use yolo_hub_client::{EndpointClient, HubEndpoint};
use yolo_hub_client_access_protocol::{
    ACCESS_REVISION_STEP, AccessConfiguration, ClientAccessChange, ClientGrant, ClientIdentity,
};
use yolo_hub_client_transport::HubPin;
use yolo_hub_identity::HubId;
use yolo_hub_routing::Right;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_ssh_entry::{ENTRY_PROFILE_SCHEMA, EntryProfile, SshEntry};

pub struct Fixture {
    pub host: Harness,
    pub hub: HubId,
    pub process: RelayProcess,
    pub sshd: SshdFixture,
    pub identity: ClientIdentity,
    pub profile: EntryProfile,
    pub profile_path: PathBuf,
    pub key_path: PathBuf,
    pub root: tempfile::TempDir,
    account_key: Arc<PrivateKey>,
    client_key: Arc<PrivateKey>,
}

impl Fixture {
    pub async fn start(rights: &[Right]) -> Self {
        let host = Harness::start().await;
        let hold = crate::hub::local_startup_support::retain_fixture_host(&host).await;
        let root = tempfile::tempdir_in(crate::hub::support::SOCKET_TEST_ROOT).unwrap();
        let path = root.path().canonicalize().unwrap();
        yolo_runtime::paths::directory::PrivateDirectory::ensure(&path).unwrap();
        let state_path = path.join(STATE_DIRECTORY);
        let state = yolo_hub_state::HubState::open(&state_path, host.home.path()).unwrap();
        let hub = state.hub_id();
        let peer = state.peer_identity();
        let socket = state.socket_path();
        let remote_socket = state
            .socket_directory()
            .unwrap()
            .path()
            .join(yolo_hub_public_client_server::PUBLIC_CLIENT_SOCKET);
        drop(state);
        let process = RelayProcess::start_existing(&host, &state_path, &socket, peer).await;
        drop(hold);
        let sshd = SshdFixture::start().await;
        let key_path = path.join(CLIENT_KEY_FILE);
        let (identity, client_key) = credentials::client_key(&key_path);
        let account_key =
            Arc::new(russh::keys::load_secret_key(sshd.identity_path(), None).unwrap());
        let profile = EntryProfile {
            schema_version: ENTRY_PROFILE_SCHEMA,
            host: sshd.address().ip().to_string(),
            port: sshd.address().port(),
            username: sshd.user().into(),
            outer_host_key: sshd.public_host_key().into(),
            remote_socket,
            hub: HubPin {
                hub_id: hub,
                public_key: *peer.key(),
            },
            client_id: identity.client_id,
        };
        profile.validate().unwrap();
        let profile_path = path.join(PROFILE_FILE);
        credentials::private_file(&profile_path, &serde_json::to_vec(&profile).unwrap());
        let fixture = Self {
            host,
            hub,
            process,
            sshd,
            identity,
            profile,
            profile_path,
            key_path,
            root,
            account_key,
            client_key,
        };
        fixture.configure(vec![fixture.grant(rights)]).await;
        fixture
    }

    pub fn entry(&self) -> Arc<SshEntry> {
        Arc::new(
            SshEntry::new(
                self.profile.clone(),
                self.account_key.clone(),
                self.client_key.clone(),
            )
            .unwrap(),
        )
    }

    pub async fn connect(&self, entry: &Arc<SshEntry>) -> EndpointClient {
        HubEndpoint::remote(entry.clone())
            .connect(self.hub, deadline())
            .await
            .unwrap_or_else(|error| {
                panic!(
                    "public SSH entry failed: {error}; {}",
                    self.sshd.diagnostics()
                )
            })
    }

    pub async fn configuration(&self) -> AccessConfiguration {
        let Reply::ClientAccess { configuration } = self
            .process
            .client()
            .await
            .call(self.hub, Command::GetClientAccess {}, deadline())
            .await
            .unwrap()
        else {
            panic!("missing client access configuration")
        };
        configuration
    }

    pub async fn configure(&self, clients: Vec<ClientGrant>) {
        let current = self.configuration().await;
        let configuration = AccessConfiguration {
            revision: current.revision + ACCESS_REVISION_STEP,
            clients,
            ..current.clone()
        };
        let Reply::ClientAccess {
            configuration: accepted,
        } = self
            .process
            .client()
            .await
            .call(
                self.hub,
                Command::ConfigureClientAccess {
                    change: ClientAccessChange {
                        request_id: uuid::Uuid::new_v4(),
                        expected_revision: current.revision,
                        configuration: configuration.clone(),
                    },
                },
                deadline(),
            )
            .await
            .unwrap()
        else {
            panic!("missing client access update acknowledgement")
        };
        assert_eq!(accepted, configuration);
    }

    pub async fn stop(self) {
        self.process.stop().await;
    }
}
