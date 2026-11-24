use super::*;

const FREEZE_SIGNAL_CONTEXT: &str = "SIGSTOP simulated power-loss target";
const FREEZE_SIGNAL_REJECTED_DETAIL: &str = "simulated power-loss target changed before SIGSTOP";
const FREEZE_TARGET_GONE_DETAIL: &str = "simulated power-loss target disappeared while freezing";
const FREEZE_TARGET_UNKNOWN_DETAIL: &str = "simulated power-loss target became indeterminate";
const FREEZE_TIMEOUT_DETAIL: &str = "simulated power-loss target did not stably stop";
const STOP_CONFIRMATIONS_REQUIRED: usize = 2;
const POWER_LOSS_FREEZE_TIMEOUT: Duration = PROCESS_EXIT_TIMEOUT;

impl PowerLossFreezeBarrier {
    pub(super) async fn freeze_exact(&mut self, identity: RuntimeProcessIdentity) -> Result<()> {
        self.freeze_exact_after_capture(identity, || {}).await
    }

    async fn freeze_exact_after_capture(
        &mut self,
        identity: RuntimeProcessIdentity,
        before_signal: impl FnOnce(),
    ) -> Result<()> {
        require_matching(identity)?;
        let already_stopped = identity.is_stably_stopped()?;
        before_signal();
        require_matching(identity)?;
        if !already_stopped {
            self.thaw_on_drop.insert(identity);
            ensure!(
                identity
                    .signal(libc::SIGSTOP)
                    .map_err(|error| anyhow::anyhow!("{FREEZE_SIGNAL_CONTEXT}: {error}"))?,
                "{FREEZE_SIGNAL_REJECTED_DETAIL}: {identity:?}"
            );
        }
        self.await_stably_stopped(identity).await?;
        self.frozen.insert(identity);
        Ok(())
    }

    async fn await_stably_stopped(&self, identity: RuntimeProcessIdentity) -> Result<()> {
        let deadline = tokio::time::Instant::now() + POWER_LOSS_FREEZE_TIMEOUT;
        let mut confirmations = 0;
        loop {
            require_matching(identity)?;
            if identity.is_stably_stopped()? {
                confirmations += 1;
                if confirmations == STOP_CONFIRMATIONS_REQUIRED {
                    return Ok(());
                }
            } else {
                confirmations = 0;
            }
            ensure!(
                tokio::time::Instant::now() < deadline,
                "{FREEZE_TIMEOUT_DETAIL}: {identity:?}"
            );
            tokio::time::sleep(PROCESS_POLL_INTERVAL).await;
        }
    }

    pub(in super::super) async fn ensure_all_frozen(&self) -> Result<()> {
        for identity in self.sorted_direct_targets() {
            ensure!(
                self.frozen.contains(&identity) && identity.is_stably_stopped()?,
                "{TARGET_NOT_FROZEN_DETAIL}: {identity:?}"
            );
            require_matching(identity)?;
        }
        ensure!(
            self.trees
                .iter()
                .all(trees::PowerLossProcessTree::is_frozen),
            TARGET_NOT_FROZEN_DETAIL
        );
        Ok(())
    }

    #[cfg(test)]
    pub(super) async fn freeze_after_capture_for_test(
        &mut self,
        identity: RuntimeProcessIdentity,
        before_signal: impl FnOnce(),
    ) -> Result<()> {
        self.freeze_exact_after_capture(identity, before_signal)
            .await
    }
}

fn require_matching(identity: RuntimeProcessIdentity) -> Result<()> {
    match identity.observe() {
        ProcessIdentityObservation::Matching => Ok(()),
        ProcessIdentityObservation::Gone => {
            anyhow::bail!("{FREEZE_TARGET_GONE_DETAIL}: {identity:?}")
        }
        ProcessIdentityObservation::Unknown => {
            anyhow::bail!("{FREEZE_TARGET_UNKNOWN_DETAIL}: {identity:?}")
        }
    }
}
