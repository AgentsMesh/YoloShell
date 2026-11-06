mod constants;
mod controller;
mod dispatch;
mod generation;
mod listener;
mod retirement;
mod transition;

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc, oneshot};
use yolo_hub_configuration_store::{CONFIGURATION_DIRECTORY, ConfigurationStore};
use yolo_hub_federation::Federation;
use yolo_hub_federation_settings::Configuration;
use yolo_hub_local_backend::LocalBackend;
use yolo_hub_management_protocol::{ConfigurationChange, ConfigurationSnapshot};
use yolo_hub_peer_dialer::{PeerDialFuture, PeerDialer};
use yolo_hub_state::HubState;
use yolo_hub_terminal_protocol::PublicError;

pub use controller::ManagementService;
type ManagementResult = Result<ConfigurationSnapshot, PublicError>;
struct ManagementRequest {
    change: Option<ConfigurationChange>,
    reply: oneshot::Sender<ManagementResult>,
}

struct NoDialer;

impl PeerDialer for NoDialer {
    fn dial<'a>(
        &'a self,
        _: yolo_hub_delegation::PeerIdentity,
        _: &'a yolo_hub_federation_settings::PeerAddress,
        _: tokio::time::Instant,
    ) -> PeerDialFuture<'a> {
        Box::pin(async { anyhow::bail!(constants::NO_DIALER) })
    }
}

pub struct ManagedHub {
    state: Arc<HubState>,
    generation: RwLock<Option<Arc<Federation>>>,
    commands: mpsc::Sender<ManagementRequest>,
    build: Option<Arc<dyn yolo_hub_dispatch::BuildProvider>>,
    dialer: Arc<dyn PeerDialer>,
}

impl ManagedHub {
    pub async fn open(
        state: Arc<HubState>,
        backend: LocalBackend,
        seed: Option<Configuration>,
    ) -> Result<(Arc<Self>, ManagementService)> {
        Self::open_with_build(state, backend, seed, None).await
    }

    pub async fn open_with_build(
        state: Arc<HubState>,
        backend: LocalBackend,
        seed: Option<Configuration>,
        build: Option<Arc<dyn yolo_hub_dispatch::BuildProvider>>,
    ) -> Result<(Arc<Self>, ManagementService)> {
        Self::open_with_dialer(state, backend, seed, build, Arc::new(NoDialer)).await
    }

    pub async fn open_with_dialer(
        state: Arc<HubState>,
        backend: LocalBackend,
        seed: Option<Configuration>,
        build: Option<Arc<dyn yolo_hub_dispatch::BuildProvider>>,
        dialer: Arc<dyn PeerDialer>,
    ) -> Result<(Arc<Self>, ManagementService)> {
        Self::open_inner(state, backend, seed, build, dialer, None).await
    }

    pub async fn open_with_origin(
        state: Arc<HubState>,
        backend: LocalBackend,
        seed: Option<Configuration>,
        build: Option<Arc<dyn yolo_hub_dispatch::BuildProvider>>,
        dialer: Arc<dyn PeerDialer>,
        origin: Arc<dyn yolo_hub_dispatch::OriginDecorator<LocalBackend>>,
    ) -> Result<(Arc<Self>, ManagementService)> {
        Self::open_inner(state, backend, seed, build, dialer, Some(origin)).await
    }

    async fn open_inner(
        state: Arc<HubState>,
        backend: LocalBackend,
        seed: Option<Configuration>,
        build: Option<Arc<dyn yolo_hub_dispatch::BuildProvider>>,
        dialer: Arc<dyn PeerDialer>,
        origin: Option<Arc<dyn yolo_hub_dispatch::OriginDecorator<LocalBackend>>>,
    ) -> Result<(Arc<Self>, ManagementService)> {
        let root = state.directory()?;
        let (commands, receiver) = mpsc::channel(constants::MANAGEMENT_QUEUE_CAPACITY);
        let store = ConfigurationStore::open(
            root.ensure_descendant(&root.path().join(CONFIGURATION_DIRECTORY))?,
            state.peer_identity(),
            seed,
        )?;
        let configuration = store.current()?;
        let listener = listener::PeerListener::publish(&state).await?;
        let prepared_backend = backend
            .prepare_files(root.path(), &configuration.configuration.file_roots)
            .await?;
        let running = generation::Running::start(
            state.clone(),
            prepared_backend,
            configuration.configuration.clone(),
            &listener,
            None,
            dialer.clone(),
            origin.clone(),
        )
        .await?;
        let hub = Arc::new(Self {
            state,
            generation: RwLock::new(Some(running.node.clone())),
            commands,
            build,
            dialer,
        });
        let service = ManagementService {
            hub: hub.clone(),
            receiver,
            store,
            backend,
            listener,
            running: Some(running),
            retiring: None,
            configuration,
            history: None,
            activation_pending: false,
            origin,
        };
        Ok((hub, service))
    }
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/lib.rs"]
mod integration_tests;
