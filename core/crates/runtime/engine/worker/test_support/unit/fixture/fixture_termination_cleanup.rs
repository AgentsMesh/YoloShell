const FIXTURE_TERMINATION_ROOT_DETAIL: &str =
    "termination fixture changed its exact child identity";
const FIXTURE_TERMINATION_OWNER_DETAIL: &str = "termination fixture owner did not settle";
const FIXTURE_SETTLED_OWNER_REFERENCES: usize = 1;

impl FixtureTerminationWork {
    async fn settle(
        mut self,
        state: Arc<WorkerState>,
        root: process::ProcessIdentity,
    ) -> Result<()> {
        anyhow::ensure!(
            state.pty_root_identity() == root,
            FIXTURE_TERMINATION_ROOT_DETAIL
        );
        let mut failure = None;
        if state.status().status != TerminalRuntimeStatus::Exited && state.pty_owner_handle().is_some() {
            let permit = state.admit_system_mutation().await;
            state.begin_system_termination(&permit, ExitReason::UserTerminate);
            failure = terminate_pty_processes(&state, permit, true).await.err();
        }
        if let Some(settlement) = self.settlement.as_ref() {
            if let Err(error) = settlement.wait().await {
                failure.get_or_insert_with(|| {
                    anyhow::anyhow!("{FIXTURE_TERMINATION_OWNER_DETAIL}: {error:?}")
                });
            }
            drop(lock_recover(&state.pty_owner).take());
        }
        if let Some(release) = self.release.as_mut() {
            if let Err(error) = release.finish_result().await {
                failure.get_or_insert(error);
            }
        }
        loop {
            anyhow::ensure!(
                state.pty_root_identity() == root,
                FIXTURE_TERMINATION_ROOT_DETAIL
            );
            let reader_settled = {
                let progress = state.reader_lifecycle.lock_progress();
                progress.has_reader_supervisor_completion() && progress.is_drained()
            };
            let owner_settled = self.settlement.as_ref().is_none_or(|settlement| {
                Arc::strong_count(settlement) == FIXTURE_SETTLED_OWNER_REFERENCES
            });
            if reader_settled
                && owner_settled
                && state.child_waited.load(Ordering::Acquire)
                && !root.matches()
                && state.status().status == TerminalRuntimeStatus::Exited
            {
                return failure.map_or(Ok(()), Err);
            }
            tokio::time::sleep(TEST_READY_POLL_INTERVAL).await;
        }
    }
}
