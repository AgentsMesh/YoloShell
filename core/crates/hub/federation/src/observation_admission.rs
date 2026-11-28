use std::{
    collections::BTreeSet,
    sync::{Mutex, OnceLock},
};
use yolo_hub_identity::HubId;

fn workers() -> &'static Mutex<BTreeSet<HubId>> {
    static WORKERS: OnceLock<Mutex<BTreeSet<HubId>>> = OnceLock::new();
    WORKERS.get_or_init(Mutex::default)
}

/// Retained by the actual worker, including across a caller's timeout or drop.
pub(crate) struct Admission(HubId);

impl Admission {
    pub fn acquire(hub: HubId) -> Option<Self> {
        let mut workers = workers().lock().unwrap_or_else(|error| error.into_inner());
        if !workers.insert(hub) {
            return None;
        }
        Some(Self(hub))
    }
}

impl Drop for Admission {
    fn drop(&mut self) {
        workers()
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .remove(&self.0);
    }
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/observation_admission.rs"]
mod unit_tests;
