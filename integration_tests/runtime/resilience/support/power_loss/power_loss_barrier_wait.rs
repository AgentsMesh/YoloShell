use super::*;

const TARGET_SURVIVED_DETAIL: &str = "frozen process identity survived simulated power loss";
const TARGET_UNKNOWN_DETAIL: &str = "simulated power-loss exit observation is indeterminate";
const POWER_LOSS_EXIT_TIMEOUT: Duration = PROCESS_EXIT_TIMEOUT;

impl PowerLossFreezeBarrier {
    pub(in super::super) async fn wait_for_gone(
        &mut self,
        paths: &Paths,
        records: &[BoundRuntimeRecord],
    ) -> Result<()> {
        let deadline = tokio::time::Instant::now() + POWER_LOSS_EXIT_TIMEOUT;
        loop {
            let mut pending = Vec::new();
            for identity in self.sorted_direct_targets() {
                match identity.observe() {
                    ProcessIdentityObservation::Gone => {}
                    ProcessIdentityObservation::Matching => pending.push(identity),
                    ProcessIdentityObservation::Unknown => {
                        anyhow::bail!("{TARGET_UNKNOWN_DETAIL}: {identity:?}")
                    }
                }
            }
            let trees_gone = self.poll_runtime_trees(deadline, paths, records).await?;
            if pending.is_empty() && trees_gone {
                self.complete();
                return Ok(());
            }
            ensure!(
                tokio::time::Instant::now() < deadline,
                "{TARGET_SURVIVED_DETAIL}: {pending:?}; trees={:?}",
                self.trees
                    .iter()
                    .map(|tree| (
                        tree.terminal_id(),
                        tree.identities(),
                        tree.authority_snapshot()
                    ))
                    .collect::<Vec<_>>()
            );
            tokio::time::sleep(PROCESS_POLL_INTERVAL).await;
        }
    }
}
