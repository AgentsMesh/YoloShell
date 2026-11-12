use super::super::super::router::{CodexRouteEntry, CodexSessionRouter, RouterTestKey};
use crate::process::ProcessIdentityObservation;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use uuid::Uuid;

type ProbeKey = (RouterTestKey, Uuid);

impl CodexSessionRouter {
    pub(in crate::worker::codex_hook) fn set_next_process_observation_for_test(
        &self,
        entry: &Arc<CodexRouteEntry>,
        observation: ProcessIdentityObservation,
    ) {
        let previous = observations()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(key(self, entry), observation);
        assert!(previous.is_none());
    }
}

pub(super) fn take(
    router: &CodexSessionRouter,
    entry: &Arc<CodexRouteEntry>,
) -> Option<ProcessIdentityObservation> {
    observations()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .remove(&key(router, entry))
}

fn observations() -> &'static Mutex<HashMap<ProbeKey, ProcessIdentityObservation>> {
    static OBSERVATIONS: OnceLock<Mutex<HashMap<ProbeKey, ProcessIdentityObservation>>> =
        OnceLock::new();
    OBSERVATIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn key(router: &CodexSessionRouter, entry: &CodexRouteEntry) -> ProbeKey {
    (router.test_key(), entry.key().invocation_id())
}
