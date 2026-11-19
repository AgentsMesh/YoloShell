pub(crate) trait PtyTerminationPermit {
    fn prepare_alias_release(
        &self,
        state: &WorkerState,
    ) -> pty_runtime_alias_closure_task::PreparedAliasClosure;
    fn commit_alias_release(
        &self,
        state: &Arc<WorkerState>,
        prepared: pty_runtime_alias_closure_task::PreparedAliasClosure,
    );
}

impl PtyTerminationPermit for ControllerMutationPermit {
    fn prepare_alias_release(
        &self,
        state: &WorkerState,
    ) -> pty_runtime_alias_closure_task::PreparedAliasClosure {
        state.prepare_controller_alias_release(self)
    }

    fn commit_alias_release(
        &self,
        state: &Arc<WorkerState>,
        prepared: pty_runtime_alias_closure_task::PreparedAliasClosure,
    ) {
        state.commit_controller_alias_release(self, prepared);
    }
}

impl PtyTerminationPermit for SystemMutationPermit {
    fn prepare_alias_release(
        &self,
        state: &WorkerState,
    ) -> pty_runtime_alias_closure_task::PreparedAliasClosure {
        state.prepare_system_alias_release(self)
    }

    fn commit_alias_release(
        &self,
        state: &Arc<WorkerState>,
        prepared: pty_runtime_alias_closure_task::PreparedAliasClosure,
    ) {
        state.commit_system_alias_release(self, prepared);
    }
}

pub(crate) async fn terminate_pty_processes<P: PtyTerminationPermit>(
    state: &Arc<WorkerState>,
    permit: P,
    force: bool,
) -> Result<()> {
    let alias_release = permit.prepare_alias_release(state);
    let submitted = match state.pty_owner_handle() {
        Some(owner) => SubmittedPtyTermination::Owner(owner.submit_termination(force)?),
        None => {
            #[cfg(test)]
            {
                SubmittedPtyTermination::TestFallback
            }
            #[cfg(not(test))]
            anyhow::bail!(PTY_OWNER_UNAVAILABLE_DETAIL)
        }
    };
    permit.commit_alias_release(state, alias_release);
    drop(permit);
    #[cfg(test)]
    state
        .control_test_hooks
        .terminate_before_delivery
        .pause_if_armed()
        .await;
    submitted.wait(state, force).await
}
