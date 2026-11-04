use super::ReapOutcome;
use anyhow::anyhow;
use std::borrow::Cow;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use std::sync::{Condvar, Mutex};

pub(crate) const COMPLETION_CHANNEL_CAPACITY: usize = 1;
pub(crate) const COMPLETION_CHANNEL_CLOSED_DETAIL: &str =
    "PTY fault reaper stopped before publishing its completion";
pub(crate) const COMPLETION_RECEIVER_DROPPED_DETAIL: &str =
    "PTY fault completion receiver stopped before accepting its outcome";
pub(crate) const DEFERRED_REAP_FAILURE_DETAIL: &str = "deferred PTY child reap failed";
pub(crate) const COMPLETION_TARGET_PANIC_DETAIL: &str = "PTY fault completion target panicked";
#[cfg(test)]
pub(crate) const INJECTED_COMPLETION_TARGET_PANIC: &str = "injected PTY completion target panic";

#[path = "completion/panic.rs"]
mod panic;
#[path = "completion/publication.rs"]
mod publication;

#[derive(Clone)]
pub(crate) struct FaultReapCompletion {
    pub(crate) shared: Arc<CompletionShared>,
}

pub(crate) struct FaultReapReceiver {
    pub(crate) receiver: Receiver<ReapOutcome>,
}

pub(crate) struct CompletionShared {
    published: AtomicBool,
    publication: Mutex<PublicationState>,
    publication_ready: Condvar,
    target: CompletionTarget,
}

pub(crate) enum CompletionTarget {
    Owner(SyncSender<ReapOutcome>),
    Finalize(Arc<crate::worker::WorkerState>),
    Log,
    #[cfg(test)]
    Panic,
    #[cfg(test)]
    PanicOnDrop,
}

pub(crate) enum PublicationState {
    Pending,
    Complete,
    TargetPanicked(Cow<'static, str>),
}

impl FaultReapCompletion {
    pub(crate) fn owner_channel() -> (Self, FaultReapReceiver) {
        let (sender, receiver) = sync_channel(COMPLETION_CHANNEL_CAPACITY);
        (
            Self::new(CompletionTarget::Owner(sender)),
            FaultReapReceiver { receiver },
        )
    }

    pub(crate) fn log() -> Self {
        Self::new(CompletionTarget::Log)
    }

    pub(crate) fn finalize(state: Arc<crate::worker::WorkerState>) -> Self {
        Self::new(CompletionTarget::Finalize(state))
    }

    #[cfg(test)]
    pub(crate) fn panicking_for_test() -> Self {
        Self::new(CompletionTarget::Panic)
    }

    #[cfg(test)]
    pub(crate) fn drop_panicking_for_test() -> Self {
        Self::new(CompletionTarget::PanicOnDrop)
    }

    #[cfg(test)]
    pub(crate) fn install_panic_trace_hook_for_test(hook: impl FnOnce() + 'static) {
        panic::install_trace_hook(hook);
    }

    pub(crate) fn is_published(&self) -> bool {
        self.shared.published.load(Ordering::Acquire)
    }

    pub(crate) fn finish_publication(&self, publication: PublicationState) {
        let mut complete = self
            .shared
            .publication
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *complete = publication;
        drop(complete);
        self.shared.publication_ready.notify_all();
    }

    pub(crate) fn wait_for_publication(&self) {
        let mut complete = self
            .shared
            .publication
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        while matches!(&*complete, PublicationState::Pending) {
            complete = self
                .shared
                .publication_ready
                .wait(complete)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        }
    }

    #[cfg(test)]
    pub(crate) fn publication_fault_for_test(&self) -> Option<String> {
        match &*self
            .shared
            .publication
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
        {
            PublicationState::TargetPanicked(detail) => Some(detail.to_string()),
            PublicationState::Pending | PublicationState::Complete => None,
        }
    }

    pub(crate) fn new(target: CompletionTarget) -> Self {
        Self {
            shared: Arc::new(CompletionShared {
                published: AtomicBool::new(false),
                publication: Mutex::new(PublicationState::Pending),
                publication_ready: Condvar::new(),
                target,
            }),
        }
    }
}

impl FaultReapReceiver {
    pub(crate) fn receive(&self) -> ReapOutcome {
        self.receiver.recv().unwrap_or_else(|_| {
            let mut outcome = ReapOutcome::lost_unknown();
            outcome.fault = Some(anyhow!(COMPLETION_CHANNEL_CLOSED_DETAIL));
            outcome
        })
    }

    #[cfg(test)]
    pub(crate) fn receive_timeout(
        &self,
        timeout: std::time::Duration,
    ) -> Result<ReapOutcome, std::sync::mpsc::RecvTimeoutError> {
        self.receiver.recv_timeout(timeout)
    }
}

pub(crate) fn log_outcome(outcome: ReapOutcome, detail: &'static str) {
    if let Some(error) = outcome.fault {
        super::super::terminal_reporting::error_error(error, detail);
    } else {
        super::super::terminal_reporting::error_message(detail);
    }
}
