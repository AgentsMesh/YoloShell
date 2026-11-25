use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ReaderRestartPhase {
    AwaitingChildExit,
    FinalDrain,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ReaderRestartKind {
    Read,
    PendingDelivery,
}

#[derive(Debug, Default)]
pub(crate) struct ReaderRecoveryBudget {
    phase: Option<ReaderRestartPhase>,
    backoff_failures: u32,
    unconfirmed_eof_reopens: u32,
    reader_failure_restarts: u32,
}

impl ReaderRecoveryBudget {
    pub(crate) fn begin_attempt(&mut self, confirmed: &AtomicBool) -> ReaderRestartPhase {
        let phase = ReaderRestartPhase::capture(confirmed);
        if self.phase != Some(phase) {
            self.phase = Some(phase);
            self.reset();
        }
        phase
    }

    pub(crate) fn reset(&mut self) {
        self.backoff_failures = u32::default();
        self.unconfirmed_eof_reopens = u32::default();
        self.reader_failure_restarts = u32::default();
    }

    pub(crate) fn record_backoff_failure(&mut self) -> u32 {
        self.backoff_failures = self.backoff_failures.saturating_add(1);
        self.backoff_failures
    }

    pub(crate) fn reset_reader_failures(&mut self) {
        self.reader_failure_restarts = u32::default();
    }

    pub(crate) fn reset_eof_reopens(&mut self) {
        self.unconfirmed_eof_reopens = u32::default();
    }

    pub(crate) fn eof_reopens_exhausted(&self, limit: u32) -> bool {
        self.unconfirmed_eof_reopens >= limit
    }

    pub(crate) fn record_eof_reopen(&mut self) {
        self.unconfirmed_eof_reopens = self.unconfirmed_eof_reopens.saturating_add(1);
    }

    pub(crate) fn reader_restarts_exhausted(&self, limit: u32) -> bool {
        self.reader_failure_restarts >= limit
    }

    pub(crate) fn record_reader_restart(&mut self) {
        self.reader_failure_restarts = self.reader_failure_restarts.saturating_add(1);
    }
}

impl ReaderRestartPhase {
    pub(crate) fn capture(confirmed: &AtomicBool) -> Self {
        if confirmed.load(Ordering::Acquire) {
            Self::FinalDrain
        } else {
            Self::AwaitingChildExit
        }
    }

    pub(crate) fn child_exit_interrupts(self, confirmed: &AtomicBool) -> bool {
        self == Self::AwaitingChildExit && confirmed.load(Ordering::Acquire)
    }
}

impl ReaderRestartKind {
    pub(crate) fn cancellation_interrupts(self, cancelled: bool) -> bool {
        self == Self::Read && cancelled
    }
}
