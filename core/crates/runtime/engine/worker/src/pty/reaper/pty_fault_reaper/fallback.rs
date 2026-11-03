use super::reservation_claim::FaultReapClaim;
use anyhow::{Context, Result, anyhow};
use std::collections::VecDeque;
use std::io;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Arc, Condvar, Mutex, OnceLock};

pub(crate) const FALLBACK_DISPATCHER_THREAD_NAME: &str = "yolo-pty-fault-fallback-dispatcher";
pub(crate) const FALLBACK_OWNER_THREAD_NAME: &str = "yolo-pty-fault-fallback-owner";
pub(crate) const FALLBACK_SPAWN_CONTEXT: &str =
    "spawn process-global PTY fault fallback dispatcher";
pub(crate) const FALLBACK_OWNER_SPAWN_FAILURE_DETAIL: &str = "spawn PTY fault fallback owner";
pub(crate) const FALLBACK_OWNER_SPAWN_PANIC_DETAIL: &str =
    "PTY fault fallback owner spawn panicked";
#[cfg(test)]
pub(crate) const INJECTED_OWNER_SPAWN_FAILURE: &str =
    "injected PTY fault fallback owner spawn failure";
#[cfg(test)]
pub(crate) const EXPECTED_OWNER_SPAWN_FAILURE: &str =
    "fallback owner spawn failure was not injected";

#[path = "fallback_spawn.rs"]
mod spawn;

pub(crate) use spawn::{OwnerSpawnFailure, spawn_owner, spawn_owner_with};

pub(crate) type FallbackTask = Box<dyn FnOnce() + Send + 'static>;
pub(crate) type TaskSlot = Arc<Mutex<Option<FallbackTask>>>;

#[derive(Clone)]
pub(crate) struct FaultReaperFallback {
    pub(crate) queue: Arc<FallbackQueue>,
}

pub(crate) struct FallbackQueue {
    pending: Mutex<VecDeque<FallbackTask>>,
    ready: Condvar,
}

pub(crate) static FALLBACK: OnceLock<std::result::Result<FaultReaperFallback, String>> =
    OnceLock::new();

impl FaultReaperFallback {
    pub(crate) fn capture() -> Result<Self> {
        FALLBACK
            .get_or_init(start_fallback)
            .as_ref()
            .map(Clone::clone)
            .map_err(|detail| anyhow!(detail.clone()))
    }

    pub(crate) fn handoff(&self, claim: FaultReapClaim) {
        self.queue.enqueue(Box::new(move || claim.reap()));
    }

    #[cfg(test)]
    pub(crate) fn handoff_task_for_test(&self, task: impl FnOnce() + Send + 'static) {
        self.queue.enqueue(Box::new(task));
    }
}

impl FallbackQueue {
    pub(crate) fn new() -> Self {
        Self {
            pending: Mutex::new(VecDeque::new()),
            ready: Condvar::new(),
        }
    }

    pub(crate) fn enqueue(&self, task: FallbackTask) {
        self.pending
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push_back(task);
        self.ready.notify_one();
    }

    pub(crate) fn receive(&self) -> FallbackTask {
        let mut pending = self
            .pending
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        loop {
            if let Some(task) = pending.pop_front() {
                return task;
            }
            pending = self
                .ready
                .wait(pending)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        }
    }
}

pub(crate) fn start_fallback() -> std::result::Result<FaultReaperFallback, String> {
    let fallback = FaultReaperFallback {
        queue: Arc::new(FallbackQueue::new()),
    };
    let queue = Arc::clone(&fallback.queue);
    std::thread::Builder::new()
        .name(FALLBACK_DISPATCHER_THREAD_NAME.to_owned())
        .spawn(move || dispatch(queue))
        .context(FALLBACK_SPAWN_CONTEXT)
        .map_err(crate::worker::supervision::error_detail)?;
    Ok(fallback)
}

pub(crate) fn dispatch(queue: Arc<FallbackQueue>) -> ! {
    loop {
        if let Err(error) = dispatch_next(&queue, spawn_owner) {
            super::super::terminal_reporting::warn_error(
                anyhow::Error::new(error),
                FALLBACK_OWNER_SPAWN_FAILURE_DETAIL,
            );
            std::thread::sleep(super::constants::PTY_FAULT_REAPER_RETRY_INITIAL_INTERVAL);
        }
    }
}

pub(crate) fn dispatch_next<Spawn>(queue: &FallbackQueue, spawn: Spawn) -> io::Result<()>
where
    Spawn: FnOnce(FallbackTask) -> std::result::Result<(), OwnerSpawnFailure>,
{
    match spawn(queue.receive()) {
        Ok(()) => Ok(()),
        Err(failure) => {
            queue.enqueue(failure.task);
            Err(failure.error)
        }
    }
}

#[cfg(test)]
pub(crate) fn recover_after_spawn_failure_for_test(
    task: impl FnOnce() + Send + 'static,
) -> FallbackTask {
    let queue = FallbackQueue::new();
    queue.enqueue(Box::new(task));
    dispatch_next(&queue, |task| {
        spawn_owner_with(task, |_| {
            Err(io::Error::other(INJECTED_OWNER_SPAWN_FAILURE))
        })
    })
    .expect_err(EXPECTED_OWNER_SPAWN_FAILURE);
    queue.receive()
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-runtime"))]
#[path = "../../../../unit_tests/pty_fault_reaper/fallback_tests.rs"]
mod tests;
