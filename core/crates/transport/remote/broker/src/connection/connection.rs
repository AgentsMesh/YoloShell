use super::connection_factory::BrokerConnectionFactory;
use super::constants::CONNECTION_GENERATION_EXHAUSTED_MESSAGE;
use super::protocol::BrokerStatus;
use crate::remote::{
    MultiplexedSshConnection, RemoteCleanupRegistry, RemoteError, RemoteRuntimeSpec,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

const CONNECTION_GENERATION_INCREMENT: u64 = 1;
const INITIAL_CONNECTION_GENERATION: u64 = 0;
#[cfg(test)]
const TEST_STATE_SIGNAL_PERMITS: usize = 1;

struct ConnectionState {
    connection: Option<Arc<dyn MultiplexedSshConnection>>,
    generation: u64,
}

#[derive(Clone)]
pub(super) struct ObservedConnection {
    connection: Arc<dyn MultiplexedSshConnection>,
    id: Uuid,
    generation: u64,
}

include!("connection_observed.rs");

pub struct BrokerConnectionPool {
    provisioning_paths: Option<super::BrokerPaths>,
    hub_transport: bool,
    hub_profile: Option<yolo_broker_hub_protocol::BrokerHubProfile>,
    instance_id: Uuid,
    host_id: String,
    config_revision: String,
    runtime: RemoteRuntimeSpec,
    factory: Arc<dyn BrokerConnectionFactory>,
    state: RwLock<ConnectionState>,
    reconnect_gate: Mutex<()>,
    retired_generation: AtomicU64,
    cleanup: RemoteCleanupRegistry,
}

impl BrokerConnectionPool {
    pub fn new(
        host_id: impl Into<String>,
        config_revision: impl Into<String>,
        runtime: RemoteRuntimeSpec,
        factory: Arc<dyn BrokerConnectionFactory>,
    ) -> Self {
        Self {
            provisioning_paths: None,
            hub_transport: false,
            hub_profile: None,
            instance_id: Uuid::new_v4(),
            host_id: host_id.into(),
            config_revision: config_revision.into(),
            runtime,
            factory,
            state: RwLock::new(ConnectionState {
                connection: None,
                generation: INITIAL_CONNECTION_GENERATION,
            }),
            reconnect_gate: Mutex::new(()),
            retired_generation: AtomicU64::new(INITIAL_CONNECTION_GENERATION),
            cleanup: RemoteCleanupRegistry::default(),
        }
    }

    pub async fn status(&self) -> BrokerStatus {
        let state = self.state.read().await;
        let retired = self.retired_generation.load(Ordering::Acquire);
        let mut status = match live_connection(&state, retired) {
            Some(connection) => self.status_for(&connection),
            None => BrokerStatus::disconnected(
                self.instance_id,
                self.host_id.clone(),
                self.config_revision.clone(),
                state.generation,
            ),
        };
        status.capabilities = self.hub_capabilities();
        status
    }

    pub(super) fn status_for(&self, observed: &ObservedConnection) -> BrokerStatus {
        BrokerStatus {
            protocol_version: super::constants::BROKER_PROTOCOL_VERSION,
            adopted: true,
            instance_id: self.instance_id,
            host_id: self.host_id.clone(),
            connection_id: Some(observed.id()),
            connection_generation: observed.generation(),
            config_revision: self.config_revision.clone(),
            capabilities: self.hub_capabilities(),
        }
    }

    pub(super) fn instance_id(&self) -> Uuid {
        self.instance_id
    }

    pub(super) fn runtime(&self) -> &RemoteRuntimeSpec {
        &self.runtime
    }

    #[cfg(test)]
    pub(super) async fn connection(
        &self,
    ) -> Result<Arc<dyn MultiplexedSshConnection>, RemoteError> {
        Ok(Arc::clone(self.observed_connection().await?.connection()))
    }

    pub(super) async fn observed_connection(&self) -> Result<ObservedConnection, RemoteError> {
        if let Some(connection) = self.cached_connection().await {
            return Ok(connection);
        }
        let _gate = self.reconnect_gate.lock().await;
        if let Some(connection) = self.cached_connection().await {
            return Ok(connection);
        }
        let connection = self
            .factory
            .connect_with_cleanup(self.cleanup.clone())
            .await?;
        let mut state = self.state.write().await;
        let retired = self.retired_generation.load(Ordering::Acquire);
        let baseline = state.generation.max(retired);
        let generation = baseline
            .checked_add(CONNECTION_GENERATION_INCREMENT)
            .ok_or_else(|| RemoteError::transport(CONNECTION_GENERATION_EXHAUSTED_MESSAGE))?;
        state.generation = generation;
        state.connection = Some(Arc::clone(&connection));
        Ok(observed(connection, generation))
    }

    pub(super) async fn cached_connection(&self) -> Option<ObservedConnection> {
        let state = self.state.read().await;
        let retired = self.retired_generation.load(Ordering::Acquire);
        live_connection(&state, retired)
    }

    pub(super) async fn invalidate(&self, observed: &ObservedConnection) {
        self.retire(observed);
        let mut state = self.state.write().await;
        let same_generation = state.generation == observed.generation();
        let same_id = state
            .connection
            .as_ref()
            .map(|connection| connection.info().connection_id)
            == Some(observed.id());
        if same_generation && same_id {
            state.connection = None;
        }
    }

    pub(super) fn retire(&self, observed: &ObservedConnection) {
        self.retired_generation
            .fetch_max(observed.generation(), Ordering::AcqRel);
    }

    pub(super) fn is_retired(&self, observed: &ObservedConnection) -> bool {
        observed.generation() <= self.retired_generation.load(Ordering::Acquire)
    }

    #[cfg(test)]
    pub(super) async fn block_state_updates(
        &self,
        entered: Arc<tokio::sync::Semaphore>,
        release: Arc<tokio::sync::Semaphore>,
    ) {
        let _state = self.state.write().await;
        entered.add_permits(TEST_STATE_SIGNAL_PERMITS);
        release.acquire().await.unwrap().forget();
    }

    #[cfg(test)]
    pub(super) async fn set_generation_for_test(&self, generation: u64) {
        let mut state = self.state.write().await;
        state.connection = None;
        state.generation = generation;
    }
}

include!("connection_identity.rs");
include!("connection_cleanup.rs");
#[path = "connection_hub_mode.rs"]
mod hub_mode;
#[path = "connection_provisioning.rs"]
mod provisioning;
