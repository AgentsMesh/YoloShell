use super::*;

#[path = "power_loss_barrier_tree_state.rs"]
mod state;
pub(super) use state::PowerLossProcessTree;

const TREE_TRANSITION_CONTEXT: &str = "advance retained simulated power-loss process tree";
const TREE_FREEZE_TIMEOUT_DETAIL: &str =
    "retained simulated power-loss process tree did not freeze";
const TREE_KILL_TIMEOUT_DETAIL: &str =
    "retained simulated power-loss process tree did not settle after kill";
const TREE_TRANSITION_TIMEOUT: Duration = PROCESS_EXIT_TIMEOUT;
const TREE_TRANSITION_BUDGET_DETAIL: &str =
    "simulated power-loss tree transition budget exceeds its supported range";

impl PowerLossFreezeBarrier {
    pub(super) fn commit_runtime_tree_kill_cleanup(&mut self) -> Result<()> {
        ensure!(
            self.trees
                .iter()
                .all(PowerLossProcessTree::kill_cleanup_can_commit),
            TREE_TRANSITION_CONTEXT
        );
        self.phase = PowerLossBarrierPhase::Killing;
        self.trees
            .iter_mut()
            .try_for_each(PowerLossProcessTree::commit_kill_cleanup)
    }

    pub(super) async fn freeze_runtime_trees(&mut self) -> Result<()> {
        let deadline = transition_deadline(self.trees.len())?;
        for index in 0..self.trees.len() {
            self.freeze_tree(index, deadline).await?;
        }
        Ok(())
    }

    pub(super) async fn kill_runtime_trees(
        &mut self,
        paths: &Paths,
        records: &[BoundRuntimeRecord],
        after_session_confirm: &mut (dyn FnMut(PowerLossTargetOwner) + Send),
    ) -> Result<()> {
        let deadline = transition_deadline(self.trees.len())?;
        for index in 0..self.trees.len() {
            self.advance_tree_to_killing(index, deadline, paths, records, after_session_confirm)
                .await?;
        }
        Ok(())
    }

    pub(super) async fn poll_runtime_trees(
        &mut self,
        deadline: tokio::time::Instant,
        paths: &Paths,
        records: &[BoundRuntimeRecord],
    ) -> Result<bool> {
        for index in 0..self.trees.len() {
            match self.trees[index]
                .poll_kill()
                .map_err(|error| anyhow::anyhow!("{TREE_TRANSITION_CONTEXT}: {error:#}"))?
            {
                state::TreeKillPoll::Gone => self.register_tree_identities(index)?,
                state::TreeKillPoll::Pending => self.register_tree_identities(index)?,
                state::TreeKillPoll::Refreeze => {
                    self.register_tree_identities(index)?;
                    let mut after_session_confirm = no_commit_boundary;
                    self.advance_tree_to_killing(
                        index,
                        deadline,
                        paths,
                        records,
                        &mut after_session_confirm,
                    )
                    .await?;
                }
            }
        }
        Ok(self.trees.iter().all(PowerLossProcessTree::is_gone))
    }

    async fn advance_tree_to_killing(
        &mut self,
        index: usize,
        deadline: tokio::time::Instant,
        paths: &Paths,
        records: &[BoundRuntimeRecord],
        after_session_confirm: &mut (dyn FnMut(PowerLossTargetOwner) + Send),
    ) -> Result<()> {
        loop {
            self.freeze_tree(index, deadline).await?;
            let owner = PowerLossTargetOwner::RuntimeTree(self.trees[index].terminal_id());
            let transition = if self.phase == PowerLossBarrierPhase::Killing {
                self.trees[index].commit_kill(|| Ok(()))
            } else {
                let record = self.record_for_owner(records, owner)?;
                self.confirm_session_authority(paths, record, SessionAuthorityPhase::Active)?;
                after_session_confirm(owner);
                let guardians = &self.guardians;
                self.trees[index].commit_kill(|| {
                    confirm_session_authority(
                        guardians,
                        paths,
                        record,
                        SessionAuthorityPhase::Active,
                    )
                })
            };
            if self.trees[index].kill_cleanup_is_armed() {
                self.commit_runtime_tree_kill_cleanup()?;
            }
            let committed = transition
                .map_err(|error| anyhow::anyhow!("{TREE_TRANSITION_CONTEXT}: {error:#}"))?;
            self.register_tree_identities(index)?;
            if committed {
                return Ok(());
            }
            require_before_deadline(deadline, TREE_KILL_TIMEOUT_DETAIL, &self.trees[index])?;
        }
    }

    async fn freeze_tree(&mut self, index: usize, deadline: tokio::time::Instant) -> Result<()> {
        loop {
            let frozen = self.trees[index]
                .poll_freeze()
                .map_err(|error| anyhow::anyhow!("{TREE_TRANSITION_CONTEXT}: {error:#}"))?;
            self.register_tree_identities(index)?;
            if frozen {
                return Ok(());
            }
            require_before_deadline(deadline, TREE_FREEZE_TIMEOUT_DETAIL, &self.trees[index])?;
            tokio::time::sleep(PROCESS_POLL_INTERVAL).await;
        }
    }

    fn register_tree_identities(&mut self, index: usize) -> Result<()> {
        let terminal_id = self.trees[index].terminal_id();
        for identity in self.trees[index].identities() {
            self.register_tree_target(identity, PowerLossTargetOwner::RuntimeTree(terminal_id))?;
        }
        Ok(())
    }
}

fn no_commit_boundary(_: PowerLossTargetOwner) {}

fn transition_budget(tree_count: usize) -> Result<Duration> {
    // Trees reconcile serially; retain the existing allowance for every participant.
    let count = u32::try_from(tree_count).context(TREE_TRANSITION_BUDGET_DETAIL)?;
    TREE_TRANSITION_TIMEOUT
        .checked_mul(count)
        .context(TREE_TRANSITION_BUDGET_DETAIL)
}

fn transition_deadline(tree_count: usize) -> Result<tokio::time::Instant> {
    tokio::time::Instant::now()
        .checked_add(transition_budget(tree_count)?)
        .context(TREE_TRANSITION_BUDGET_DETAIL)
}

#[cfg(test)]
#[path = "power_loss_transition_budget_tests.rs"]
mod budget_tests;

fn require_before_deadline(
    deadline: tokio::time::Instant,
    detail: &str,
    tree: &PowerLossProcessTree,
) -> Result<()> {
    ensure!(
        tokio::time::Instant::now() < deadline,
        "{detail}: session_id={}, identities={:?}, authority={}",
        tree.terminal_id(),
        tree.identities(),
        tree.authority_snapshot(),
    );
    Ok(())
}
