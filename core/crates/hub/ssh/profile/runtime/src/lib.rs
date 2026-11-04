mod active;
mod constants;
mod dial;
mod dispatcher;
mod launch;
mod monitor;
mod reservations;
use anyhow::{Result, ensure};
pub use launch::profile_binding;
pub use reservations::ProfileReservation;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::sync::watch;
use yolo_hub_dispatch::Dispatcher;
use yolo_hub_ssh_profiles::{SSH_PROFILES_DIRECTORY, SshProfileConfiguration, SshProfileStore};
use yolo_hub_state::HubState;

/// The persistent Hub owns profile authority; brokers retain SSH and credential custody.
pub struct SshProfileRuntime {
    state: Arc<HubState>,
    store: Arc<Mutex<SshProfileStore>>,
    changed: watch::Sender<u64>,
    active: Arc<Mutex<Vec<active::ActiveTunnel>>>,
    launcher: Option<BrokerExecutable>,
    reservations: Arc<Mutex<reservations::Reservations>>,
}

#[derive(Clone)]
pub struct BrokerExecutable {
    pub executable: PathBuf,
    pub host_home: PathBuf,
}

impl SshProfileRuntime {
    pub fn open(state: Arc<HubState>, seed: Option<SshProfileConfiguration>) -> Result<Arc<Self>> {
        Self::open_with_launcher(state, seed, None)
    }
    pub fn open_with_launcher(
        state: Arc<HubState>,
        seed: Option<SshProfileConfiguration>,
        launcher: Option<BrokerExecutable>,
    ) -> Result<Arc<Self>> {
        let root = state.directory()?;
        let store = SshProfileStore::open(
            root.ensure_descendant(&root.path().join(SSH_PROFILES_DIRECTORY))?,
            state.peer_identity(),
            seed,
        )?;
        let (changed, _) = watch::channel(store.current()?.revision);
        Ok(Arc::new(Self {
            state,
            store: Arc::new(Mutex::new(store)),
            changed,
            active: Arc::new(Mutex::new(Vec::new())),
            launcher,
            reservations: Arc::new(Mutex::new(reservations::Reservations::default())),
        }))
    }
    pub fn dispatcher(self: &Arc<Self>, inner: Arc<dyn Dispatcher>) -> Result<Arc<dyn Dispatcher>> {
        ensure!(
            inner.hub_id() == self.state.hub_id(),
            constants::REGISTRY_CHANGED
        );
        Ok(Arc::new(dispatcher::ProfileDispatcher {
            profiles: self.clone(),
            inner,
        }))
    }
}

impl Drop for SshProfileRuntime {
    fn drop(&mut self) {
        self.close_all();
    }
}
