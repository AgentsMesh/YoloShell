use super::constants::{FULL_RIGHTS, PRIVATE_FILE_MODE};
use crate::hub::federation_support::RunningNode;
use crate::ssh_transport::SshdFixture;
use ring::signature::{Ed25519KeyPair, KeyPair};
use russh::keys::{PrivateKey, pkcs8::decode_pkcs8};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::{sync::oneshot, task::JoinHandle};
use yolo_hub_client_access_protocol::{
    ACCESS_SCHEMA_VERSION, AccessConfiguration, ClientGrant, ClientIdentity, FIRST_ACCESS_REVISION,
    OriginAccess,
};
use yolo_hub_client_access_registry::AccessRegistry;
use yolo_hub_client_transport::HubPin;
use yolo_hub_dispatch::Dispatcher;
use yolo_hub_identity::HubId;
use yolo_hub_public_client_server::PublicClientServer;
use yolo_hub_ssh_entry::{ENTRY_PROFILE_SCHEMA, EntryProfile, SshEntry};
use yolo_hub_state::HubState;

pub struct SharedClient {
    pub identity: ClientIdentity,
    pub key: Arc<PrivateKey>,
    pub key_path: PathBuf,
}

impl SharedClient {
    pub fn generate(directory: &Path) -> Self {
        let document = Ed25519KeyPair::generate_pkcs8(&ring::rand::SystemRandom::new()).unwrap();
        let signing = Ed25519KeyPair::from_pkcs8(document.as_ref()).unwrap();
        let key = Arc::new(decode_pkcs8(document.as_ref(), None).unwrap());
        let directory = directory.canonicalize().unwrap();
        yolo_runtime::paths::directory::PrivateDirectory::ensure(&directory).unwrap();
        let key_path = directory.join(super::constants::MULTIHOP_KEY_FILE);
        let encoded = key
            .to_openssh(russh::keys::ssh_key::LineEnding::LF)
            .unwrap();
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(PRIVATE_FILE_MODE)
            .open(&key_path)
            .unwrap();
        file.write_all(encoded.as_bytes()).unwrap();
        file.sync_all().unwrap();
        Self {
            identity: ClientIdentity {
                client_id: uuid::Uuid::new_v4(),
                public_key: signing.public_key().as_ref().try_into().unwrap(),
            },
            key,
            key_path,
        }
    }
}

pub struct Ingress {
    pub profile: EntryProfile,
    pub entry: Arc<SshEntry>,
    stop: Option<oneshot::Sender<()>>,
    task: Option<JoinHandle<anyhow::Result<()>>>,
    _sshd: SshdFixture,
}

impl Ingress {
    pub async fn start(
        node: &RunningNode,
        state: Arc<HubState>,
        client: &SharedClient,
        origins: &[HubId],
    ) -> Self {
        let sshd = SshdFixture::start().await;
        let rights: yolo_hub_routing::Rights = FULL_RIGHTS.into_iter().collect();
        let origins = origins
            .iter()
            .copied()
            .map(|origin| OriginAccess {
                origin,
                rights: rights.clone(),
            })
            .collect();
        let registry = AccessRegistry::new(AccessConfiguration {
            schema_version: ACCESS_SCHEMA_VERSION,
            entry_hub: node.node.hub_id(),
            revision: FIRST_ACCESS_REVISION,
            clients: vec![ClientGrant {
                identity: client.identity,
                origins,
                expires_at_ms: yolo_runtime::model::now_ms() as u64
                    + super::constants::GRANT_LIFETIME.as_millis() as u64,
            }],
        })
        .unwrap();
        let dispatcher: Arc<dyn Dispatcher> = node.node.clone();
        let server = PublicClientServer::bind(state.clone(), dispatcher, registry)
            .await
            .unwrap();
        let socket = server.socket_path().to_path_buf();
        let (stop, stopped) = oneshot::channel();
        let task = tokio::spawn(server.serve_until(async {
            let _ = stopped.await;
        }));
        let profile = EntryProfile {
            schema_version: ENTRY_PROFILE_SCHEMA,
            host: sshd.address().ip().to_string(),
            port: sshd.address().port(),
            username: sshd.user().to_owned(),
            outer_host_key: sshd.public_host_key().to_owned(),
            remote_socket: socket,
            hub: HubPin {
                hub_id: node.node.hub_id(),
                public_key: state.public_key().try_into().unwrap(),
            },
            client_id: client.identity.client_id,
        };
        let entry = Arc::new(
            SshEntry::new(
                profile.clone(),
                Arc::new(russh::keys::load_secret_key(sshd.identity_path(), None).unwrap()),
                client.key.clone(),
            )
            .unwrap(),
        );
        Self {
            profile,
            entry,
            stop: Some(stop),
            task: Some(task),
            _sshd: sshd,
        }
    }

    pub async fn close(mut self) {
        self.entry.close(super::deadline()).await.unwrap();
        self.stop.take().unwrap().send(()).unwrap();
        self.task.take().unwrap().await.unwrap().unwrap();
    }

    pub fn account_key_path(&self) -> &Path {
        self._sshd.identity_path()
    }

    pub fn write_profile(&self, path: &Path) {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let bytes = serde_json::to_vec(&self.profile).unwrap();
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(PRIVATE_FILE_MODE)
            .open(path)
            .unwrap();
        file.write_all(&bytes).unwrap();
        file.sync_all().unwrap();
    }
}

impl Drop for Ingress {
    fn drop(&mut self) {
        let _ = self.stop.take().map(|stop| stop.send(()));
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}
