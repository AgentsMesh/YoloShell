use super::output_block::OutputBlockCause;
use super::{WorkerState, lock_recover, supervision};
use std::sync::Arc;
use std::time::{Duration, Instant};

const PTY_OUTPUT_BLOCKED_AFTER: Duration = Duration::from_secs(30);
const PTY_OUTPUT_BLOCKED_DETAIL: &str =
    "PTY exit finalization is blocked on durable output delivery";
const PTY_READER_DRAIN_WAIT_DETAIL: &str =
    "waiting for PTY output to reach the durable journal before recording exit";

#[derive(Clone, Copy)]
pub(super) struct FinalizationSchedule {
    pub(super) drain_interval: Duration,
    pub(super) blocked_after: Duration,
}

impl FinalizationSchedule {
    pub(super) fn production(drain_interval: Duration) -> Self {
        Self {
            drain_interval,
            blocked_after: PTY_OUTPUT_BLOCKED_AFTER,
        }
    }
}

pub(super) fn await_reader_drain(state: &Arc<WorkerState>, schedule: FinalizationSchedule) {
    let blocked_deadline = Instant::now() + schedule.blocked_after;
    loop {
        let now = Instant::now();
        let wait = if now >= blocked_deadline {
            schedule.drain_interval
        } else {
            schedule
                .drain_interval
                .min(blocked_deadline.saturating_duration_since(now))
        };
        if !drain_reader(state, wait) {
            return;
        }
        if Instant::now() >= blocked_deadline
            && state.mark_output_blocked(OutputBlockCause::FinalDrainDeadline)
        {
            let _ = supervision::contain(|| {
                tracing::error!(terminal_id = %state.terminal_id(), PTY_OUTPUT_BLOCKED_DETAIL);
            });
        }
        let _ = supervision::contain(|| {
            tracing::warn!(
                terminal_id = %state.terminal_id(),
                timeout_ms = wait.as_millis(),
                PTY_READER_DRAIN_WAIT_DETAIL
            );
        });
    }
}

pub(super) fn drain_reader(state: &WorkerState, timeout: Duration) -> bool {
    let progress = lock_recover(&state.reader_lifecycle.progress);
    if progress.is_drained() {
        state.clear_output_blocked(OutputBlockCause::FinalDrainDeadline);
        return false;
    }
    let waited = state
        .reader_lifecycle
        .changed
        .wait_timeout_while(progress, timeout, |value| !value.is_drained());
    let (progress, _) = waited.unwrap_or_else(|poisoned| {
        let _ = supervision::contain(|| {
            tracing::error!("recovering poisoned PTY reader condition state");
        });
        poisoned.into_inner()
    });
    let pending = !progress.is_drained();
    drop(progress);
    if !pending {
        state.clear_output_blocked(OutputBlockCause::FinalDrainDeadline);
    }
    pending
}
