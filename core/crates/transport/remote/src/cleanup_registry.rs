use super::RemoteError;
use super::constants::{
    CLEANUP_REGISTRY_CLOSED_MESSAGE, CLEANUP_REGISTRY_EXHAUSTED_MESSAGE,
    CLEANUP_REGISTRY_FAILED_MESSAGE, SSH_CLEANUP_PHASE_COUNT, SSH_CLEANUP_RESERVATION_INCREMENT,
    SSH_CLEANUP_SETTLEMENT_MARGIN,
};
use std::future::Future;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::{Mutex as AsyncMutex, Notify};
use tokio::task::JoinHandle;

type CleanupResult = Result<(), RemoteError>;
type CleanupTask = JoinHandle<CleanupResult>;

#[derive(Clone, Copy, Eq, PartialEq)]
enum RegistryPhase {
    Open,
    Settling,
    Complete,
    Failed,
}

struct RegistryState {
    phase: RegistryPhase,
    reservations: usize,
    failures: usize,
    tasks: Vec<CleanupTask>,
    stages: std::collections::BTreeMap<uuid::Uuid, StageEntry>,
}

struct RegistryInner {
    state: Mutex<RegistryState>,
    settlement_gate: AsyncMutex<()>,
    changed: Notify,
    #[cfg(test)]
    test: RegistryTestState,
}

#[derive(Clone)]
pub struct RemoteCleanupRegistry {
    inner: Arc<RegistryInner>,
}

pub struct RemoteCleanupReservation {
    registry: Option<RemoteCleanupRegistry>,
}

impl Default for RemoteCleanupRegistry {
    fn default() -> Self {
        Self {
            inner: Arc::new(RegistryInner {
                state: Mutex::new(RegistryState {
                    phase: RegistryPhase::Open,
                    reservations: usize::default(),
                    failures: usize::default(),
                    tasks: Vec::new(),
                    stages: std::collections::BTreeMap::new(),
                }),
                settlement_gate: AsyncMutex::new(()),
                changed: Notify::new(),
                #[cfg(test)]
                test: RegistryTestState::default(),
            }),
        }
    }
}

impl RemoteCleanupRegistry {
    pub(crate) fn reserve(&self) -> Result<RemoteCleanupReservation, RemoteError> {
        let mut state = self.inner.state.lock().unwrap();
        if state.phase != RegistryPhase::Open {
            return Err(RemoteError::transport(CLEANUP_REGISTRY_CLOSED_MESSAGE));
        }
        state.reservations = state
            .reservations
            .checked_add(SSH_CLEANUP_RESERVATION_INCREMENT)
            .ok_or_else(|| RemoteError::transport(CLEANUP_REGISTRY_EXHAUSTED_MESSAGE))?;
        Ok(RemoteCleanupReservation {
            registry: Some(self.clone()),
        })
    }

    fn release_reservation(&self) {
        let mut state = self.inner.state.lock().unwrap();
        consume_reservation(&mut state);
        drop(state);
        self.inner.changed.notify_one();
    }

    fn launch<F>(&self, future: F)
    where
        F: Future<Output = CleanupResult> + Send + 'static,
    {
        let runtime = tokio::runtime::Handle::try_current();
        let mut state = self.inner.state.lock().unwrap();
        consume_reservation(&mut state);
        if state.phase == RegistryPhase::Complete {
            state.failures = state
                .failures
                .saturating_add(SSH_CLEANUP_RESERVATION_INCREMENT);
        } else if let Ok(runtime) = runtime {
            #[cfg(test)]
            let future = cleanup_after_barrier(
                future,
                self.inner.test.next_task_barrier.lock().unwrap().take(),
            );
            state.tasks.push(runtime.spawn(future));
        } else {
            state.failures = state
                .failures
                .saturating_add(SSH_CLEANUP_RESERVATION_INCREMENT);
        }
        drop(state);
        self.inner.changed.notify_one();
    }

    fn terminal_result(&self) -> Option<CleanupResult> {
        let state = self.inner.state.lock().unwrap();
        match state.phase {
            RegistryPhase::Complete => Some(Ok(())),
            RegistryPhase::Failed => {
                Some(Err(RemoteError::transport(CLEANUP_REGISTRY_FAILED_MESSAGE)))
            }
            RegistryPhase::Open | RegistryPhase::Settling => None,
        }
    }
}

fn consume_reservation(state: &mut RegistryState) {
    if let Some(remaining) = state
        .reservations
        .checked_sub(SSH_CLEANUP_RESERVATION_INCREMENT)
    {
        state.reservations = remaining;
    } else {
        state.failures = state
            .failures
            .saturating_add(SSH_CLEANUP_RESERVATION_INCREMENT);
    }
}

impl RemoteCleanupReservation {
    pub(crate) fn launch<F>(mut self, future: F)
    where
        F: Future<Output = CleanupResult> + Send + 'static,
    {
        if let Some(registry) = self.registry.take() {
            registry.launch(future);
        }
    }
}

impl Drop for RemoteCleanupReservation {
    fn drop(&mut self) {
        if let Some(registry) = self.registry.take() {
            registry.release_reservation();
        }
    }
}

impl Drop for RegistryInner {
    fn drop(&mut self) {
        let state = self
            .state
            .get_mut()
            .unwrap_or_else(|error| error.into_inner());
        if state.reservations != 0 || state.failures != 0 || !state.tasks.is_empty() {
            tracing::error!(
                reservations = state.reservations,
                failures = state.failures,
                tasks = state.tasks.len(),
                "SSH cleanup registry dropped before settlement"
            );
        }
    }
}

pub const fn cleanup_settlement_timeout(operation_timeout: Duration) -> Duration {
    operation_timeout
        .saturating_mul(SSH_CLEANUP_PHASE_COUNT)
        .saturating_add(SSH_CLEANUP_SETTLEMENT_MARGIN)
}

include!("cleanup_registry_task_custody.rs");
include!("cleanup_registry_settlement.rs");
include!("cleanup_registry_stages.rs");
#[cfg(feature = "broker-test-support")]
include!("cleanup_registry_broker_test_support.rs");

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
include!("../integration_tests/cleanup_registry_test_barrier.rs");

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../integration_tests/cleanup_registry_tests.rs"]
mod tests;
