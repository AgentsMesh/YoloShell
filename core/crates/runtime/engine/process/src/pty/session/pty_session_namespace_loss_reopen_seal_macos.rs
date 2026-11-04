use super::macos_passive_master::MacosPassivePtyMaster;
use super::macos_reopen_seal::{MacosCommittedPtyReopenSeal, MacosPtyReopenSeal};
use anyhow::{Context, Result};
use std::sync::Mutex;

const LOCK_CONTEXT: &str = "lock namespace-loss PTY reopen seal";

#[cfg(any(
    all(test, feature = "yolo-integration-tests"),
    feature = "adversarial-test-hooks"
))]
#[path = "../../../test_support/namespace/namespace_loss_reopen_seal_hook.rs"]
mod test_hook;
#[cfg(any(
    all(test, feature = "yolo-integration-tests"),
    feature = "adversarial-test-hooks"
))]
pub use test_hook::install as install_pre_finalize_hook;

pub(in crate::process) struct MacosNamespaceLossReopenSeal {
    committed: Mutex<Option<MacosPtyReopenSeal>>,
}

pub(in crate::process) enum MacosNamespaceLossFinalize {
    Holders(Vec<crate::process::ProcessIdentity>),
    Finalized(super::macos_reopen_seal::MacosFinalizedPtyReopenSeal),
}

impl MacosNamespaceLossReopenSeal {
    pub(in crate::process) fn new() -> Self {
        Self {
            committed: Mutex::new(None),
        }
    }

    pub(in crate::process) fn ensure_committed(
        &self,
        passive: &MacosPassivePtyMaster,
    ) -> Result<MacosCommittedPtyReopenSeal> {
        let mut committed = self
            .committed
            .lock()
            .map_err(|_| anyhow::anyhow!(LOCK_CONTEXT))?;
        ensure_seal(&mut committed, passive)?.committed_passive(passive)
    }

    pub(in crate::process) fn containment_snapshot(
        &self,
        passive: &MacosPassivePtyMaster,
        exclusions: &super::macos_passive_master::MacosPtyHolderExclusions,
    ) -> Result<(
        MacosCommittedPtyReopenSeal,
        Vec<crate::process::ProcessIdentity>,
    )> {
        let mut committed = self
            .committed
            .lock()
            .map_err(|_| anyhow::anyhow!(LOCK_CONTEXT))?;
        let seal = ensure_seal(&mut committed, passive)?;
        let receipt = seal.committed_passive(passive)?;
        let holders = seal.capture_passive_holders(passive, exclusions)?;
        Ok((receipt, holders))
    }

    pub(in crate::process) fn finalize_if_quiescent(
        &self,
        passive: &MacosPassivePtyMaster,
        expected: &MacosCommittedPtyReopenSeal,
        exclusions: &super::macos_passive_master::MacosPtyHolderExclusions,
    ) -> Result<MacosNamespaceLossFinalize> {
        let mut committed = self
            .committed
            .lock()
            .map_err(|_| anyhow::anyhow!(LOCK_CONTEXT))?;
        let seal = ensure_seal(&mut committed, passive)?;
        let current = seal.committed_passive(passive)?;
        anyhow::ensure!(current.same_authority(expected), LOCK_CONTEXT);
        let holders = seal.capture_passive_holders(passive, exclusions)?;
        if !holders.is_empty() {
            return Ok(MacosNamespaceLossFinalize::Holders(holders));
        }
        #[cfg(any(
            all(test, feature = "yolo-integration-tests"),
            feature = "adversarial-test-hooks"
        ))]
        test_hook::invoke()?;
        let holders = seal.capture_passive_holders(passive, exclusions)?;
        if !holders.is_empty() {
            return Ok(MacosNamespaceLossFinalize::Holders(holders));
        }
        seal.finalize_passive(passive)
            .map(MacosNamespaceLossFinalize::Finalized)
    }
}

fn ensure_seal<'a>(
    committed: &'a mut Option<MacosPtyReopenSeal>,
    passive: &MacosPassivePtyMaster,
) -> Result<&'a mut MacosPtyReopenSeal> {
    if committed.is_none() {
        let mut seal = MacosPtyReopenSeal::prepare_passive(passive)?;
        seal.activate()?;
        seal.commit_passive(passive)?;
        *committed = Some(seal);
    }
    committed.as_mut().context(LOCK_CONTEXT)
}
