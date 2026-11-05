use crate::{
    constants::*, status::ConsumerRegistry, observation_cache::ObservationCache, registry::Link,
};
use anyhow::{Result, ensure};
use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64},
    },
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::{Mutex, RwLock, watch};
use yolo_agent_provider_contract::ProjectionPolicies;
use yolo_hub_dispatch::{Dispatcher, OriginDecorator};
use yolo_hub_federation_directory::OriginBook;
use yolo_hub_federation_settings::{Configuration, routing_limits};
use yolo_hub_identity::HubId;
use yolo_hub_local_backend::LocalBackend;
use yolo_hub_peer_diagnostics::ExportDepthDenials;
use yolo_hub_peer_dialer::{PeerDialFuture, PeerDialer};
use yolo_hub_peer_protocol::{Catalogue, Signed};
use yolo_hub_publication::Publication;
use yolo_hub_state::HubState;

struct NoDialer;

impl PeerDialer for NoDialer {
    fn dial<'a>(
        &'a self,
        _: yolo_hub_delegation::PeerIdentity,
        _: &'a yolo_hub_federation_settings::PeerAddress,
        _: tokio::time::Instant,
    ) -> PeerDialFuture<'a> {
        Box::pin(async { anyhow::bail!(NO_DIALER) })
    }
}

pub struct Federation {
    pub(crate) state: Arc<HubState>,
    pub(crate) backend: LocalBackend,
    pub(crate) origin: Arc<dyn Dispatcher>,
    pub(crate) config: Configuration,
    pub(crate) book: Mutex<OriginBook>,
    pub(crate) observation_cache: ObservationCache,
    pub(crate) export_denials: ExportDepthDenials,
    pub(crate) connections: RwLock<BTreeMap<HubId, Arc<Link>>>,
    pub(crate) publication: Mutex<Publication>,
    pub(crate) latest: watch::Sender<Option<Arc<Signed<Catalogue>>>>,
    pub(crate) catalogue_wake: watch::Sender<u64>,
    pub(crate) policy_epoch: u64,
    pub(crate) directory_revision: AtomicU64,
    pub(crate) closing: AtomicBool,
    pub(crate) lifetime: Arc<crate::lifetime::Lifetime>,
    pub(crate) dialer: Arc<dyn PeerDialer>,
    pub(crate) consumers: ConsumerRegistry,
}
impl Federation {
    pub async fn new(
        state: Arc<HubState>,
        backend: LocalBackend,
        config: Configuration,
    ) -> Result<Arc<Self>> {
        Self::new_with_dialer(state, backend, config, Arc::new(NoDialer), None).await
    }

    pub async fn new_with_dialer(
        state: Arc<HubState>,
        backend: LocalBackend,
        config: Configuration,
        dialer: Arc<dyn PeerDialer>,
        origin: Option<Arc<dyn OriginDecorator<LocalBackend>>>,
    ) -> Result<Arc<Self>> {
        ensure!(state.hub_id() == backend.hub_id(), IDENTITY_MISMATCH);
        let identity = state.peer_identity();
        config.validate(identity)?;
        ensure!(
            backend.files_match(&config.file_roots),
            FILE_ROOTS_UNPREPARED
        );
        let root = state.directory()?;
        let backend = backend.with_layouts(&root)?;
        let backend = if backend.agents_enabled() {
            backend
        } else {
            backend.with_agents(
                root.ensure_descendant(&root.path().join(AGENT_DIRECTORY))?,
                ProjectionPolicies::none(),
            )?
        };
        let publication = Publication::open(
            root.ensure_descendant(&root.path().join(PUBLICATION_DIRECTORY))?,
            identity,
            &config,
        )?;
        let policy_epoch = publication.policy_epoch();
        let (latest, _) = watch::channel(publication.latest().cloned().map(Arc::new));
        let (catalogue_wake, _) = watch::channel(0);
        let mut book = OriginBook::new(identity, config.pins(), routing_limits())?;
        let observation_cache = ObservationCache::open(state.clone(), &mut book).await;
        let origin: Arc<dyn Dispatcher> = origin.map_or_else(
            || Arc::new(backend.clone()) as Arc<dyn Dispatcher>,
            |decorator| decorator.decorate(backend.clone()),
        );
        Ok(Arc::new(Self {
            origin,
            state,
            backend,
            config,
            book: Mutex::new(book),
            observation_cache,
            export_denials: ExportDepthDenials::default(),
            connections: RwLock::new(BTreeMap::new()),
            publication: Mutex::new(publication),
            latest,
            catalogue_wake,
            policy_epoch,
            directory_revision: AtomicU64::new(0),
            closing: AtomicBool::new(false),
            lifetime: Arc::default(),
            dialer,
            consumers: ConsumerRegistry::default(),
        }))
    }
    pub fn hub_id(&self) -> HubId {
        self.state.hub_id()
    }
    pub fn source(&self) -> Option<Arc<Signed<Catalogue>>> {
        if self.require_open().is_err() {
            return None;
        }
        self.latest.borrow().clone()
    }
}
pub(crate) fn now_ms() -> Result<u64> {
    Ok(u64::try_from(
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis(),
    )?)
}
