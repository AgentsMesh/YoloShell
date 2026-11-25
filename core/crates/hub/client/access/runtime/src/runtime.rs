use crate::{constants::*, custody::StoreGuard};
use anyhow::{Result, ensure};
use std::sync::{Arc, Mutex};
use yolo_hub_client_access_protocol::AccessConfiguration;
use yolo_hub_client_access_registry::AccessRegistry;
use yolo_hub_client_access_store::{CLIENT_ACCESS_DIRECTORY, ClientAccessStore};
use yolo_hub_dispatch::Dispatcher;
use yolo_hub_state::HubState;

pub struct ClientAccessRuntime {
    pub(crate) state: Arc<HubState>,
    pub(crate) inner: Arc<dyn Dispatcher>,
    pub(crate) store: Mutex<ClientAccessStore>,
    pub(crate) registry: AccessRegistry,
}

impl ClientAccessRuntime {
    pub fn open(
        state: Arc<HubState>,
        inner: Arc<dyn Dispatcher>,
        seed: Option<AccessConfiguration>,
    ) -> Result<Arc<Self>> {
        ensure!(state.hub_id() == inner.hub_id(), IDENTITY_MISMATCH);
        let root = state.directory()?;
        let directory = root.ensure_descendant(&root.path().join(CLIENT_ACCESS_DIRECTORY))?;
        let store = ClientAccessStore::open(directory, state.hub_id(), seed)?;
        let registry = AccessRegistry::new(store.current()?)?;
        let runtime = Arc::new(Self {
            state,
            inner,
            store: Mutex::new(store),
            registry,
        });
        runtime.current()?;
        Ok(runtime)
    }

    pub fn registry(&self) -> AccessRegistry {
        self.registry.clone()
    }

    pub(crate) fn current(&self) -> Result<AccessConfiguration> {
        let result = (|| {
            let store = self.lock_store()?;
            self.current_locked(&store)
        })();
        self.fail_closed(result)
    }

    pub(crate) fn current_locked(&self, store: &ClientAccessStore) -> Result<AccessConfiguration> {
        self.state.verify()?;
        let configuration = store.current()?;
        ensure!(configuration == self.registry.current()?, AUTHORITY_CHANGED);
        self.state.verify()?;
        Ok(configuration)
    }

    pub(crate) fn lock_store(&self) -> Result<StoreGuard<'_>> {
        let store = self.store.lock().map_err(|_| {
            self.registry.close();
            anyhow::anyhow!(STORAGE_POISONED)
        })?;
        Ok(StoreGuard {
            store,
            registry: &self.registry,
        })
    }

    pub(crate) fn fail_closed<T>(&self, result: Result<T>) -> Result<T> {
        if result.is_err() {
            self.registry.close();
        }
        result
    }
}

impl Drop for ClientAccessRuntime {
    fn drop(&mut self) {
        self.registry.close();
    }
}
