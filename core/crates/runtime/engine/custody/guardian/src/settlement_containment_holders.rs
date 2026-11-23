use super::{ExactTarget, NamespaceLossContainment, NamespaceLossExactSettlement};
use crate::process::{MacosSecondaryCustody, ProcessIdentity};
use anyhow::{Context, Result, ensure};
use tokio::time::Instant;

const HOLDER_LIMIT_DETAIL: &str = "namespace-loss PTY holder authority exceeds its bound";
const HOLDER_RESERVE_CONTEXT: &str = "reserve namespace-loss PTY holder authority";

impl NamespaceLossContainment {
    pub(in crate::custody_guardian) fn reconcile_custody(
        &mut self,
        custody: &MacosSecondaryCustody,
        phase_now: Instant,
    ) -> Result<Option<NamespaceLossExactSettlement>> {
        self.initialize_custody(custody)?;
        if !self.reconcile(phase_now)? {
            return Ok(None);
        }
        if let Some(finalized) = self.finalized.clone() {
            self.refresh_finalized_holders(custody)?;
            if !self.reconcile(phase_now)? {
                return Ok(None);
            }
            return Ok(Some(finalized));
        }
        let finalization = custody.finalize_namespace_loss_reopen_seal(self.authority()?)?;
        match finalization {
            crate::process::MacosNamespaceLossSealFinalization::Holders(holders) => {
                self.track(holders)?;
                Ok(None)
            }
            crate::process::MacosNamespaceLossSealFinalization::Finalized(finalized) => {
                let settlement = NamespaceLossExactSettlement::new(finalized);
                self.finalized = Some(settlement.clone());
                let discovery_clock = tokio::time::Instant::now();
                self.late_discovery.arm(discovery_clock);
                Ok(Some(settlement))
            }
        }
    }

    pub(super) fn track(
        &mut self,
        identities: impl IntoIterator<Item = ProcessIdentity>,
    ) -> Result<()> {
        self.prune_settled_holders()?;
        let mut additions = Vec::new();
        for identity in identities {
            if self.worker.identity == identity
                || self.child.identity == identity
                || self
                    .holders
                    .iter()
                    .any(|target| target.identity == identity)
                || additions.contains(&identity)
            {
                continue;
            }
            additions.push(identity);
        }
        ensure!(
            self.holders.len().saturating_add(additions.len())
                <= crate::constants::MAX_NAMESPACE_LOSS_PTY_HOLDERS,
            HOLDER_LIMIT_DETAIL
        );
        self.holders
            .try_reserve(additions.len())
            .context(HOLDER_RESERVE_CONTEXT)?;
        self.holders
            .extend(additions.into_iter().map(ExactTarget::new));
        Ok(())
    }

    fn prune_settled_holders(&mut self) -> Result<()> {
        let mut outcome = Ok(());
        let mut index = usize::default();
        while index < self.holders.len() {
            match self.holders[index].observe_settled() {
                Ok(true) => {
                    self.holders.swap_remove(index);
                }
                Ok(false) => index += 1,
                Err(error) => {
                    retain_error(&mut outcome, Err(error));
                    index += 1;
                }
            }
        }
        outcome
    }

    pub(super) fn exact_settlement(&mut self) -> Result<bool> {
        if !matches!(self.phase, super::Phase::Force) {
            return Ok(false);
        }
        let mut outcome = Ok(());
        let mut settled = true;
        observe_target(&mut self.worker, &mut settled, &mut outcome);
        observe_target(&mut self.child, &mut settled, &mut outcome);
        let mut holder_index = usize::default();
        while holder_index < self.holders.len() {
            match self.holders[holder_index].observe_settled() {
                Ok(true) => {
                    self.holders.swap_remove(holder_index);
                }
                Ok(false) => {
                    settled = false;
                    holder_index += 1;
                }
                Err(error) => {
                    settled = false;
                    retain_error(&mut outcome, Err(error));
                    holder_index += 1;
                }
            }
        }
        outcome?;
        Ok(settled && self.holders.is_empty())
    }

    #[cfg(test)]
    pub(in crate::custody_guardian) fn tracks_holder(&self, identity: ProcessIdentity) -> bool {
        self.holders
            .iter()
            .any(|target| target.identity == identity)
    }

    #[cfg(test)]
    pub(in crate::custody_guardian) fn holder_discovery_scans_for_test(&self) -> usize {
        self.holder_discovery_scans
    }
}

fn observe_target(target: &mut ExactTarget, settled: &mut bool, outcome: &mut Result<()>) {
    match target.observe_settled() {
        Ok(observed) => *settled &= observed,
        Err(error) => {
            *settled = false;
            retain_error(outcome, Err(error));
        }
    }
}

pub(super) fn retain_error(outcome: &mut Result<()>, result: Result<()>) {
    let Err(error) = result else { return };
    *outcome = match std::mem::replace(outcome, Ok(())) {
        Ok(()) => Err(error),
        Err(previous) => Err(previous.context(format!(
            "{}: {error:#}",
            super::TARGET_RECONCILIATION_CONTEXT
        ))),
    };
}
