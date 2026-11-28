pub(crate) const PTY_RETAINED_OUTPUT_FAILURE_CONTEXT: &str =
    "persist pending PTY output through retained journal authority";
pub(crate) const PTY_RETAINED_APPEND_ATTEMPT_MISSING_DETAIL: &str =
    "pending output has no journal append attempt";

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ReaderLoopOutcome {
    ChildExited,
    Cancelled,
    UnexpectedEof { output_observed: bool },
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum PendingDelivery {
    Delivered,
    NamespaceLost,
}

#[derive(Debug)]
pub(crate) enum PendingCommitFailure {
    Ordinary { error: anyhow::Error, attempt: u32 },
    RetainedDefinitelyUncommitted { error: anyhow::Error, attempt: u32 },
    RetainedCommitUncertain { error: anyhow::Error, attempt: u32 },
}

impl PendingCommitFailure {
    pub(crate) fn into_error(self) -> anyhow::Error {
        match self {
            Self::Ordinary { error, .. }
            | Self::RetainedDefinitelyUncommitted { error, .. }
            | Self::RetainedCommitUncertain { error, .. } => error,
        }
    }
}

pub(crate) struct ReaderAttemptGuard<'a> {
    lifecycle: &'a ReaderLifecycle,
}

impl<'a> ReaderAttemptGuard<'a> {
    pub(crate) fn start(lifecycle: &'a ReaderLifecycle) -> Option<Self> {
        lifecycle.begin_attempt().then_some(Self { lifecycle })
    }
}

impl Drop for ReaderAttemptGuard<'_> {
    fn drop(&mut self) {
        self.lifecycle.finish_attempt();
    }
}

pub(crate) fn run_reader_attempt(
    reader: Box<dyn Read + Send>,
    state: Arc<WorkerState>,
) -> Result<ReaderLoopOutcome> {
    let Some(_attempt) = ReaderAttemptGuard::start(&state.reader_lifecycle) else {
        return Ok(ReaderLoopOutcome::Cancelled);
    };
    reader_loop(reader, Arc::clone(&state))
}
include!("../output/pty_reader_loop.rs");
include!("../output/pty_pending_retry.rs");

#[cfg(test)]
pub(crate) fn pty_slave_closed(error: &std::io::Error) -> bool {
    pty_output_terminal::is_kernel_pty_eof_error(error)
}

#[cfg(test)]
pub(crate) fn commit_reader_output(state: &WorkerState, output: &[u8]) -> Result<()> {
    stage_reader_output(state, output)?;
    match commit_staged_reader_output(state).map_err(PendingCommitFailure::into_error)? {
        PendingDelivery::Delivered => Ok(()),
        PendingDelivery::NamespaceLost => bail!(PTY_READER_NAMESPACE_LOST_DETAIL),
    }
}

#[cfg(test)]
pub(crate) fn stage_reader_output(state: &WorkerState, output: &[u8]) -> Result<()> {
    let attempt = state.prepare_output_append(output.len())?;
    stage_reader_output_with_attempt(state, output, attempt)
}

#[cfg(test)]
pub(crate) fn stage_reader_output_with_attempt(
    state: &WorkerState,
    output: &[u8],
    attempt: AppendAttempt,
) -> Result<()> {
    let mut progress = lock_recover(&state.reader_lifecycle.progress);
    anyhow::ensure!(
        progress.pending.is_empty(),
        PTY_PENDING_OUTPUT_CONFLICT_DETAIL
    );
    progress.pending.push(reader_lifecycle::PendingOutput {
        data: output.to_vec(),
        attempt: Some(attempt),
    });
    progress.eof.clear();
    progress.kernel_eof_probe_identity = None;
    progress.kernel_eof_seal_epoch = None;
    progress.pending_commit_retries = PendingCommitRetryBudget::default();
    progress.drained_probe_epoch = None;
    state.reader_lifecycle.changed.notify_all();
    Ok(())
}
include!("../output/pty_output_delivery.rs");
