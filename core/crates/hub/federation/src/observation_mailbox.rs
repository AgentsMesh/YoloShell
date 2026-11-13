use std::sync::{
    Condvar, Mutex,
    atomic::{AtomicBool, Ordering},
};
use tokio::sync::oneshot;
use yolo_hub_federation_directory::ObservationHistory;
use yolo_hub_observation_store::Retirement;

pub(crate) struct Pending {
    pub history: ObservationHistory,
    pub completed: oneshot::Sender<()>,
}

#[derive(Default)]
pub(crate) struct Mailbox {
    pub retirement: Retirement,
    pub available: AtomicBool,
    pending: Mutex<Option<Pending>>,
    changed: Condvar,
}

impl Mailbox {
    pub fn activate(&self) -> bool {
        let _pending = self
            .pending
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if self.retirement.is_retired() {
            return false;
        }
        self.available.store(true, Ordering::Release);
        true
    }

    pub fn record(&self, history: ObservationHistory) -> Option<oneshot::Receiver<()>> {
        let mut pending = self
            .pending
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if self.available.load(Ordering::Acquire) && !self.retirement.is_retired() {
            let (completed, receipt) = oneshot::channel();
            *pending = Some(Pending { history, completed });
            self.changed.notify_one();
            Some(receipt)
        } else {
            None
        }
    }

    pub fn next(&self) -> Option<Pending> {
        let mut pending = self
            .pending
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        loop {
            if self.retirement.is_retired() {
                return None;
            }
            if let Some(history) = pending.take() {
                return Some(history);
            }
            pending = self
                .changed
                .wait(pending)
                .unwrap_or_else(|error| error.into_inner());
        }
    }

    pub fn retire(&self) {
        self.retirement.retire();
        let mut pending = self
            .pending
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        self.available.store(false, Ordering::Release);
        pending.take();
        self.changed.notify_all();
    }
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/observation_mailbox.rs"]
mod unit_tests;
