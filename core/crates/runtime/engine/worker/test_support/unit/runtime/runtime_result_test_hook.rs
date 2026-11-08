use anyhow::Result;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use uuid::Uuid;

type Observations = HashMap<Uuid, Option<String>>;

static OBSERVATIONS: OnceLock<Mutex<Observations>> = OnceLock::new();

pub(super) fn watch(terminal_id: Uuid) {
    locked().insert(terminal_id, None);
}

pub(super) fn record(terminal_id: Uuid, result: &Result<()>) {
    let Err(error) = result else {
        return;
    };
    record_pending(terminal_id, &format!("{error:#}"));
}

pub(super) fn record_pending(terminal_id: Uuid, detail: &str) {
    if let Some(observation) = locked().get_mut(&terminal_id) {
        *observation = Some(detail.to_owned());
    }
}

pub(super) fn take(terminal_id: Uuid) -> Option<String> {
    locked().remove(&terminal_id).flatten()
}

fn observations() -> &'static Mutex<Observations> {
    OBSERVATIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn locked() -> std::sync::MutexGuard<'static, Observations> {
    observations()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(all(feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))]
#[path = "../../../unit_tests/runtime/runtime_result_test_hook_tests.rs"]
mod tests;
