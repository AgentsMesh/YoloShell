use crate::protocol::ErrorBody;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::sync::{Notify, OwnedRwLockReadGuard, OwnedRwLockWriteGuard, RwLock};
use tokio::time::Instant;

const HOST_DRAINING: &str = "host is draining for idle shutdown";
const HOST_DRAIN_CONFLICT: &str = "host idle shutdown is already in progress";
const HOST_FAIL_STOPPED: &str = "host is stopping after an unrecoverable custody fault";
const HOST_RESPONSE_DRAIN_OVERFLOW: &str = "host response publication count overflowed";
const HOST_RESPONSE_DRAIN_UNDERFLOW: &str =
    "host response publication resolved without a reservation";
const HOST_RESPONSE_ADMISSION_CLOSED: &str = "host response admission is closed for shutdown";
const HOST_FAIL_STOP_LOG: &str = "Yolo host fail-stop committed";

#[derive(Clone, Default)]
pub(super) struct HostRequestDrain(Arc<HostRequestDrainState>);

#[derive(Default)]
struct HostRequestDrainState {
    draining: AtomicBool,
    fail_stopped: AtomicBool,
    graceful_shutdown: AtomicBool,
    fail_stop_detail: Mutex<Option<String>>,
    mutations: Arc<RwLock<()>>,
    responses: Mutex<HostResponseState>,
    responses_changed: Notify,
    shutdown: Notify,
}

#[derive(Default)]
struct HostResponseState {
    pending: usize,
    shutdown_committed: bool,
}

pub(super) struct HostMutationPermit {
    _guard: OwnedRwLockReadGuard<()>,
}

pub(super) struct HostDrainPreparation {
    state: Arc<HostRequestDrainState>,
    _guard: OwnedRwLockWriteGuard<()>,
    committed: bool,
}

impl HostRequestDrain {
    pub(super) fn admit_response(&self) -> std::result::Result<HostResponseTicket, ErrorBody> {
        let mut responses = super::lock_recover(&self.0.responses);
        if responses.shutdown_committed {
            return Err(self.response_admission_error());
        }
        responses.pending = responses
            .pending
            .checked_add(1)
            .expect(HOST_RESPONSE_DRAIN_OVERFLOW);
        Ok(HostResponseTicket {
            state: Some(Arc::clone(&self.0)),
        })
    }

    pub(super) async fn admit_mutation(
        &self,
    ) -> std::result::Result<HostMutationPermit, ErrorBody> {
        if let Some(error) = self.admission_error() {
            return Err(error);
        }
        let guard = Arc::clone(&self.0.mutations).read_owned().await;
        if let Some(error) = self.admission_error() {
            return Err(error);
        }
        Ok(HostMutationPermit { _guard: guard })
    }

    pub(super) fn ensure_attachment_open(&self) -> std::result::Result<(), ErrorBody> {
        match self.admission_error() {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    pub(super) async fn prepare(&self) -> std::result::Result<HostDrainPreparation, ErrorBody> {
        if self.is_fail_stopped() {
            return Err(self.fail_stopped_error());
        }
        let guard = Arc::clone(&self.0.mutations).write_owned().await;
        if self.is_fail_stopped() {
            return Err(self.fail_stopped_error());
        }
        self.0
            .draining
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| ErrorBody::conflict(HOST_DRAIN_CONFLICT))?;
        Ok(HostDrainPreparation {
            state: Arc::clone(&self.0),
            _guard: guard,
            committed: false,
        })
    }

    #[cfg(test)]
    pub(super) fn is_draining(&self) -> bool {
        self.0.draining.load(Ordering::Acquire) || self.is_fail_stopped()
    }

    pub(super) fn fail_stop(&self, detail: impl Into<String>) {
        let detail = detail.into();
        let mut responses = super::lock_recover(&self.0.responses);
        let mut stored = super::lock_recover(&self.0.fail_stop_detail);
        if stored.is_none() {
            tracing::error!(detail = %detail, "{HOST_FAIL_STOP_LOG}");
            *stored = Some(detail);
        }
        drop(stored);
        self.0.fail_stopped.store(true, Ordering::Release);
        responses.shutdown_committed = true;
        drop(responses);
        self.0.responses_changed.notify_waiters();
        self.0.shutdown.notify_one();
    }

    pub(super) fn commit_response_shutdown(&self) {
        super::lock_recover(&self.0.responses).shutdown_committed = true;
        self.0.responses_changed.notify_waiters();
    }

    pub(super) async fn wait_for_response_drain(&self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        loop {
            let notified = self.0.responses_changed.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            if super::lock_recover(&self.0.responses).pending == 0 {
                return true;
            }
            let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                return false;
            };
            if tokio::time::timeout(remaining, &mut notified)
                .await
                .is_err()
            {
                return false;
            }
        }
    }
}

impl HostDrainPreparation {
    pub(super) fn commit(mut self) {
        self.committed = true;
        self.state.graceful_shutdown.store(true, Ordering::Release);
        self.state.shutdown.notify_one();
    }
}

impl Drop for HostDrainPreparation {
    fn drop(&mut self) {
        if !self.committed {
            self.state.draining.store(false, Ordering::Release);
        }
    }
}

fn draining_error() -> ErrorBody {
    ErrorBody::conflict(HOST_DRAINING)
}

include!("request_drain_state.rs");
include!("request_response_ticket.rs");
