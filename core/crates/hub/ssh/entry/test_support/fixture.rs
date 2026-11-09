use super::*;
use russh::{keys::PrivateKey, server};
use std::sync::Arc;
use tokio::{
    net::TcpListener,
    sync::{Mutex, mpsc},
    task::JoinHandle,
    time::timeout_at,
};
use uuid::Uuid;
use yolo_hub_client_access_protocol::{
    ACCESS_SCHEMA_VERSION, AccessConfiguration, ClientGrant, ClientIdentity, FIRST_ACCESS_REVISION,
    OriginAccess,
};
use yolo_hub_client_access_registry::AccessRegistry;
use yolo_hub_client_transport::HubPin;
use yolo_hub_identity::HubId;
use yolo_hub_routing::Right;
use yolo_hub_ssh_entry::{ENTRY_PROFILE_SCHEMA, EntryProfile, SshEntry};

pub struct Fixture {
    pub profile: EntryProfile,
    pub outer_key: Arc<PrivateKey>,
    pub inner_key: Arc<PrivateKey>,
    pub registry: AccessRegistry,
    pub counts: Arc<Counts>,
    admitted: Mutex<mpsc::UnboundedReceiver<server::Handle>>,
    listener: JoinHandle<()>,
}
impl Fixture {
    pub async fn start(service: Service) -> Self {
        let (_, host_key) = keys::keys();
        let (_, outer_key) = keys::keys();
        let (hub_public, hub_key) = keys::keys();
        let (client_public, inner_key) = keys::keys();
        let listener = TcpListener::bind((HOST, u16::default())).await.unwrap();
        let hub = HubPin {
            hub_id: HubId::new(),
            public_key: hub_public,
        };
        let client_id = Uuid::new_v4();
        let registry = AccessRegistry::new(AccessConfiguration {
            schema_version: ACCESS_SCHEMA_VERSION,
            entry_hub: hub.hub_id,
            revision: FIRST_ACCESS_REVISION,
            clients: vec![ClientGrant {
                identity: ClientIdentity {
                    client_id,
                    public_key: client_public,
                },
                origins: vec![OriginAccess {
                    origin: hub.hub_id,
                    rights: [Right::TerminalRead].into_iter().collect(),
                }],
                expires_at_ms: now_ms() + u64::try_from(GRANT_LIFETIME.as_millis()).unwrap(),
            }],
        })
        .unwrap();
        let profile = EntryProfile {
            schema_version: ENTRY_PROFILE_SCHEMA,
            host: HOST.into(),
            port: listener.local_addr().unwrap().port(),
            username: USERNAME.into(),
            outer_host_key: host_key.public_key().to_openssh().unwrap(),
            remote_socket: SOCKET.into(),
            hub,
            client_id,
        };
        let counts = Arc::new(Counts::default());
        let (admitted, receiver) = mpsc::unbounded_channel();
        let shared = outer::Shared {
            account: outer_key.public_key().clone(),
            hub_key,
            registry: registry.clone(),
            counts: counts.clone(),
            service,
            admitted,
        };
        Self {
            profile,
            outer_key,
            inner_key,
            registry,
            counts,
            admitted: Mutex::new(receiver),
            listener: tokio::spawn(outer::listen(listener, host_key, shared)),
        }
    }
    pub fn entry(&self) -> SshEntry {
        self.with_profile(self.profile.clone())
    }
    pub fn with_profile(&self, profile: EntryProfile) -> SshEntry {
        SshEntry::new(profile, self.outer_key.clone(), self.inner_key.clone()).unwrap()
    }
    pub async fn admitted(&self) -> server::Handle {
        timeout_at(deadline(), self.admitted.lock().await.recv())
            .await
            .unwrap()
            .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.registry.close();
        self.listener.abort();
    }
}
