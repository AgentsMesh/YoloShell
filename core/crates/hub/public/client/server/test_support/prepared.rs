use super::*;
use yolo_hub_dispatch::Dispatcher;

/// Unpublished state permits identity rejection checks before any socket exists.
pub struct Prepared {
    pub state: Arc<HubState>,
    pub backend: Arc<Backend>,
    pub registry: AccessRegistry,
    pub key: Arc<russh::keys::PrivateKey>,
    directory: tempfile::TempDir,
    home: tempfile::TempDir,
}

impl Prepared {
    pub fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let state =
            Arc::new(HubState::open(&directory.path().join(HUB_DIRECTORY), home.path()).unwrap());
        let document = Ed25519KeyPair::generate_pkcs8(&ring::rand::SystemRandom::new()).unwrap();
        let signing = Ed25519KeyPair::from_pkcs8(document.as_ref()).unwrap();
        let key = Arc::new(russh::keys::pkcs8::decode_pkcs8(document.as_ref(), None).unwrap());
        let backend = Arc::new(Backend {
            hub: state.hub_id(),
            allowed: HubId::new(),
            hidden: HubId::new(),
            calls: Default::default(),
        });
        let registry = AccessRegistry::new(AccessConfiguration {
            schema_version: ACCESS_SCHEMA_VERSION,
            entry_hub: state.hub_id(),
            revision: FIRST_ACCESS_REVISION,
            clients: vec![ClientGrant {
                identity: ClientIdentity {
                    client_id: uuid::Uuid::new_v4(),
                    public_key: signing.public_key().as_ref().try_into().unwrap(),
                },
                origins: vec![OriginAccess {
                    origin: backend.allowed,
                    rights: [Right::Observe].into_iter().collect(),
                }],
                expires_at_ms: now_ms() + GRANT_LIFETIME_MS,
            }],
        })
        .unwrap();
        Self {
            state,
            backend,
            registry,
            key,
            directory,
            home,
        }
    }
    pub fn socket(&self) -> PathBuf {
        self.state
            .socket_directory()
            .unwrap()
            .path()
            .join(yolo_hub_public_client_server::PUBLIC_CLIENT_SOCKET)
    }
    pub async fn start(self, dispatcher: Arc<dyn Dispatcher>) -> Fixture {
        let server =
            PublicClientServer::bind(self.state.clone(), dispatcher, self.registry.clone())
                .await
                .unwrap();
        let socket = server.socket_path().to_owned();
        let (stop, stopped) = oneshot::channel();
        let task = tokio::spawn(server.serve_until(async {
            let _ = stopped.await;
        }));
        Fixture {
            state: self.state,
            backend: self.backend,
            registry: self.registry,
            socket,
            key: self.key,
            stop: Some(stop),
            task: Some(task),
            _directory: self.directory,
            _home: self.home,
        }
    }
}
