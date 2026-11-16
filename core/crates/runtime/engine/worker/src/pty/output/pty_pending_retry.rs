pub(crate) const INITIAL_PENDING_RETRY_ATTEMPT: u32 = 1;
pub(crate) const PTY_RETAINED_OUTPUT_UNCERTAIN_DETAIL: &str =
    "retained PTY output commit remains uncertain; pending output is preserved";

#[derive(Debug, Default)]
pub(crate) struct PendingCommitRetryBudget {
    pub(crate) ordinary_failures: u32,
    retained_failures: u32,
}

impl PendingCommitRetryBudget {
    pub(crate) fn record_ordinary_failure(&mut self) -> u32 {
        self.ordinary_failures = self.ordinary_failures.saturating_add(1);
        self.ordinary_failures
    }

    pub(crate) fn record_retained_definitely_uncommitted(&mut self) -> u32 {
        self.retained_failures = self.retained_failures.saturating_add(1);
        self.retained_failures
    }

    pub(crate) fn record_retained_commit_uncertain(&mut self) -> u32 {
        self.retained_failures = self.retained_failures.saturating_add(1);
        self.retained_failures
    }
}

pub(crate) fn deliver_pending_with_retry(state: &Arc<WorkerState>) -> Result<PendingDelivery> {
    loop {
        let (error, attempt) = match commit_staged_reader_output(state) {
            Ok(outcome) => return Ok(outcome),
            Err(PendingCommitFailure::Ordinary { error, attempt }) => (error, attempt),
            Err(PendingCommitFailure::RetainedDefinitelyUncommitted {
                error,
                attempt,
            }) => (error, attempt),
            Err(PendingCommitFailure::RetainedCommitUncertain { error, attempt }) => {
                tracing::error!(
                    %error,
                    retry_attempts = attempt,
                    PTY_RETAINED_OUTPUT_UNCERTAIN_DETAIL
                );
                (error, attempt)
            }
        };
        let delay = supervision::RestartPolicy::default().delay(attempt);
        tracing::warn!(
            %error,
            retry_delay_ms = delay.as_millis(),
            PTY_OUTPUT_RETRY_DETAIL
        );
        std::thread::sleep(delay);
    }
}
