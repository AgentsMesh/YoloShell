pub(crate) const PTY_MASTER_UNAVAILABLE_CONTEXT: &str =
    "PTY master unavailable while resolving foreground process group";
pub(crate) const PTY_FOREGROUND_MISSING_CONTEXT: &str = "PTY has no foreground process group";
pub(crate) const PTY_FOREGROUND_CHANGED_DETAIL: &str =
    "PTY foreground process group changed before signal delivery";
pub(crate) const PTY_FOREGROUND_IDENTITY_CHANGED_DETAIL: &str =
    "PTY foreground process-group identity changed before signal delivery";
pub(crate) const PTY_FORCE_SIGNAL_REQUIRES_TERMINATION_DETAIL: &str =
    "SIGKILL requires the typed force-termination operation";
#[cfg(not(test))]
pub(crate) const PTY_OWNER_UNAVAILABLE_DETAIL: &str = "PTY lifetime owner is unavailable";
pub(crate) const PTY_QUERY_TASK_CONTEXT: &str = "PTY process query task failed";
pub(crate) const PTY_TERMINATION_SUBMISSION_MISSING_DETAIL: &str =
    "live PTY termination has no submitted owner command";

#[derive(Clone, Copy)]
pub(crate) struct PtyForegroundSignal(i32);

impl PtyForegroundSignal {
    pub(crate) fn parse(name: &str) -> Result<Self> {
        let signal = process::parse_signal(name)?;
        anyhow::ensure!(
            signal != libc::SIGKILL,
            PTY_FORCE_SIGNAL_REQUIRES_TERMINATION_DETAIL
        );
        Ok(Self(signal))
    }
}

impl WorkerState {
    pub(crate) fn pty_root_identity(&self) -> process::ProcessIdentity {
        let record = lock_recover(&self.record);
        process::ProcessIdentity {
            pid: record.child_pid,
            start_time: record.child_start_time,
        }
    }

    pub(crate) fn foreground_process_group(&self) -> Result<i32> {
        lock_recover(&self.master)
            .as_ref()
            .context(PTY_MASTER_UNAVAILABLE_CONTEXT)?
            .process_group_leader()
            .context(PTY_FOREGROUND_MISSING_CONTEXT)
    }
}

pub(crate) async fn signal_pty_foreground(
    state: &Arc<WorkerState>,
    _permit: &WriterMutationPermit,
    signal: PtyForegroundSignal,
) -> Result<()> {
    let root = state.pty_root_identity();
    let group_id = state.foreground_process_group()?;
    let group = run_process_query(move || {
        process::PtyProcessScope::capture(root)?.foreground_group(group_id)
    })
    .await?;
    #[cfg(test)]
    state
        .control_test_hooks
        .signal_before_delivery
        .pause_if_armed()
        .await;
    anyhow::ensure!(
        state.foreground_process_group()? == group_id,
        PTY_FOREGROUND_CHANGED_DETAIL
    );
    anyhow::ensure!(
        process::signal_process_group(group, signal.0)?,
        PTY_FOREGROUND_IDENTITY_CHANGED_DETAIL
    );
    Ok(())
}

include!("pty_process_termination.rs");

pub(crate) enum SubmittedPtyTermination {
    Owner(pty_owner_handle::PtyTerminationSubmission),
    #[cfg(test)]
    TestFallback,
}

impl SubmittedPtyTermination {
    pub(crate) async fn wait(self, _state: &Arc<WorkerState>, _force: bool) -> Result<()> {
        match self {
            Self::Owner(submission) => submission.wait().await,
            #[cfg(test)]
            Self::TestFallback => terminate_submitted_process_scope(_state, _force).await,
        }
    }
}

pub(crate) fn preflight_pty_termination(
    state: &Arc<WorkerState>,
    _permit: &ControllerMutationPermit,
) -> Result<()> {
    if let Some(owner) = state.pty_owner_handle() {
        return owner.preflight_destructive();
    }
    #[cfg(test)]
    {
        let root = state.pty_root_identity();
        let tree = process::CapturedProcessTree::capture(root)?;
        tree.require_destructive_session()
            .map_err(anyhow::Error::new)?;
        return Ok(());
    }
    #[cfg(not(test))]
    anyhow::bail!(PTY_OWNER_UNAVAILABLE_DETAIL)
}

pub(crate) fn submit_pty_termination(
    state: &Arc<WorkerState>,
    permit: &ControllerMutationPermit,
    force: bool,
) -> Result<SubmittedPtyTermination> {
    if let Some(owner) = state.pty_owner_handle() {
        let alias_release = state.prepare_controller_alias_release(permit);
        let submission = owner
            .submit_termination(force)
            .map(SubmittedPtyTermination::Owner)?;
        state.commit_controller_alias_release(permit, alias_release);
        return Ok(submission);
    }
    #[cfg(test)]
    {
        let alias_release = state.prepare_controller_alias_release(permit);
        state.commit_controller_alias_release(permit, alias_release);
        return Ok(SubmittedPtyTermination::TestFallback);
    }
    #[cfg(not(test))]
    anyhow::bail!(PTY_OWNER_UNAVAILABLE_DETAIL)
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
include!("../../../test_support/integration/pty/pty_process_test_support.rs");

pub(crate) async fn run_process_query<T: Send + 'static>(
    query: impl FnOnce() -> Result<T> + Send + 'static,
) -> Result<T> {
    tokio::task::spawn_blocking(query)
        .await
        .context(PTY_QUERY_TASK_CONTEXT)?
}
