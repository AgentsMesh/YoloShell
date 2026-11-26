use std::{
    ops::{Deref, DerefMut},
    sync::MutexGuard,
};
use yolo_hub_client_access_registry::AccessRegistry;
use yolo_hub_client_access_store::ClientAccessStore;

pub(crate) struct StoreGuard<'a> {
    pub store: MutexGuard<'a, ClientAccessStore>,
    pub registry: &'a AccessRegistry,
}

impl Deref for StoreGuard<'_> {
    type Target = ClientAccessStore;
    fn deref(&self) -> &Self::Target {
        &self.store
    }
}

impl DerefMut for StoreGuard<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.store
    }
}

impl Drop for StoreGuard<'_> {
    fn drop(&mut self) {
        if std::thread::panicking() {
            self.registry.close();
        }
    }
}

pub(crate) struct MonitorGuard(pub AccessRegistry);

impl Drop for MonitorGuard {
    fn drop(&mut self) {
        self.0.close();
    }
}
