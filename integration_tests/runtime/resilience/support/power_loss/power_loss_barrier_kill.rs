use super::*;

const TARGET_KILL_CONTEXT: &str = "SIGKILL frozen simulated power-loss target";
const TARGET_KILL_REJECTED_DETAIL: &str = "frozen simulated power-loss target changed";
const TARGET_KILL_FAILURE_SEPARATOR: &str = "; ";

impl PowerLossFreezeBarrier {
    pub(in super::super) async fn kill_all(
        &mut self,
        paths: &Paths,
        records: &[BoundRuntimeRecord],
    ) -> Result<()> {
        self.kill_all_with_commit_boundary(paths, records, |_| {})
            .await
    }

    #[cfg(test)]
    pub(in super::super) async fn kill_all_after_session_confirm_for_test(
        &mut self,
        paths: &Paths,
        records: &[BoundRuntimeRecord],
        boundary: PowerLossCommitBoundary,
        after_session_confirm: impl FnOnce() + Send,
    ) -> Result<()> {
        let mut pending = Some(after_session_confirm);
        self.kill_all_with_commit_boundary(paths, records, move |owner| {
            if boundary.matches(owner)
                && let Some(after_session_confirm) = pending.take()
            {
                after_session_confirm();
            }
        })
        .await
    }

    async fn kill_all_with_commit_boundary(
        &mut self,
        paths: &Paths,
        records: &[BoundRuntimeRecord],
        mut after_session_confirm: impl FnMut(PowerLossTargetOwner) + Send,
    ) -> Result<()> {
        self.confirm_guardians(paths)?;
        self.ensure_all_frozen().await?;
        super::super::validation::validate_targets(paths, records)?;
        self.kill_runtime_trees(paths, records, &mut after_session_confirm)
            .await?;
        self.commit_runtime_tree_kill_cleanup()?;
        self.kill_direct_targets(paths, records, &mut after_session_confirm)
            .await
    }

    async fn kill_direct_targets(
        &mut self,
        paths: &Paths,
        records: &[BoundRuntimeRecord],
        after_session_confirm: &mut (dyn FnMut(PowerLossTargetOwner) + Send),
    ) -> Result<()> {
        let mut failures = Vec::new();
        let mut killed = Vec::new();
        for (identity, owner) in self.sorted_direct_target_entries() {
            let record = self.record_for_owner(records, owner)?;
            self.confirm_session_authority(paths, record, SessionAuthorityPhase::Retained)?;
            after_session_confirm(owner);
            ensure!(
                self.frozen.contains(&identity) && identity.is_stably_stopped()?,
                "{TARGET_NOT_FROZEN_DETAIL}: {identity:?}"
            );
            confirm_session_authority(
                &self.guardians,
                paths,
                record,
                SessionAuthorityPhase::Retained,
            )?;
            match identity.signal(libc::SIGKILL) {
                Ok(true) => killed.push(identity),
                Ok(false) => failures.push(format!("{TARGET_KILL_REJECTED_DETAIL}: {identity:?}")),
                Err(error) => {
                    failures.push(format!("{TARGET_KILL_CONTEXT}: {identity:?}: {error:#}"))
                }
            }
        }
        for identity in killed {
            wait_for_identity_gone(ProcessIdentity {
                pid: identity.pid,
                start_time: identity.start_time,
            })
            .await;
            discard_incomplete_coverage_profile(identity.pid);
        }
        if failures.is_empty() {
            Ok(())
        } else {
            anyhow::bail!(failures.join(TARGET_KILL_FAILURE_SEPARATOR))
        }
    }
}
