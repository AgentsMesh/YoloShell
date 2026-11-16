use super::ProcessEvents;
use crate::process::{LiveProcessIdentity, ProcessIdentity, UnreapedRootExit};

const PROCESS_EVENT_INVENTORY_EPOCH_INCREMENT: u64 = 1;

#[derive(Clone, Copy, Default, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct ProcessEventInventoryEpoch(u64);

pub(in crate::process) struct ExactLineageContainmentAuthority {
    allowed_fork_versions: std::collections::HashMap<u64, u64>,
    fork_event_version: u64,
    inventory_epoch: ProcessEventInventoryEpoch,
}

impl ProcessEventInventoryEpoch {
    fn next(self) -> Option<Self> {
        self.0
            .checked_add(PROCESS_EVENT_INVENTORY_EPOCH_INCREMENT)
            .map(Self)
    }
}

impl ProcessEvents {
    #[cfg(target_os = "macos")]
    pub fn unresolved_fork_versions(&self) -> std::collections::HashMap<u64, u64> {
        self.gate.as_ref().map_or_else(
            std::collections::HashMap::new,
            super::MacosProcessEvents::unresolved_fork_versions,
        )
    }

    #[cfg(not(target_os = "macos"))]
    pub fn unresolved_fork_versions(&self) -> std::collections::HashMap<u64, u64> {
        std::collections::HashMap::new()
    }

    #[cfg(target_os = "macos")]
    pub(in crate::process) fn exact_root_exit_proof(
        &mut self,
        root: ProcessIdentity,
        live_root: LiveProcessIdentity,
    ) -> Option<UnreapedRootExit> {
        self.requires_reconciliation();
        self.gate
            .as_ref()
            .is_some_and(|gate| gate.exact_root_exited(live_root))
            .then(|| UnreapedRootExit::from_exact_exit_event(root, live_root))
    }

    pub(in crate::process) fn inventory_publication_is_current(&self) -> bool {
        self.publication_fence == Some(self.reconciliation_epoch)
    }

    pub(super) fn begin_inventory_reconciliation(&mut self) -> bool {
        self.publication_fence = None;
        let Some(next) = self.reconciliation_epoch.next() else {
            self.pending_reconciliation = true;
            return false;
        };
        self.reconciliation_epoch = next;
        true
    }

    pub(super) fn record_inventory_publication(&mut self, quiescent: bool) {
        self.publication_fence = quiescent.then_some(self.reconciliation_epoch);
    }

    pub(super) fn invalidate_inventory_publication(&mut self) {
        self.publication_fence = None;
    }

    pub(in crate::process) fn exact_lineage_containment_authority(
        &self,
        allowed_fork_versions: &std::collections::HashMap<u64, u64>,
    ) -> Option<ExactLineageContainmentAuthority> {
        #[cfg(target_os = "macos")]
        {
            if self.pending_reconciliation || self.reconciliation_requires_followup {
                return None;
            }
            let inventory_epoch = self
                .publication_fence
                .filter(|epoch| *epoch == self.reconciliation_epoch)?;
            let gate = self.gate.as_ref()?;
            let (allowed_fork_versions, fork_event_version) =
                gate.exact_containment_snapshot(allowed_fork_versions)?;
            Some(ExactLineageContainmentAuthority {
                allowed_fork_versions,
                fork_event_version,
                inventory_epoch,
            })
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = allowed_fork_versions;
            None
        }
    }

    pub(in crate::process) fn acknowledge_exact_containment(
        &mut self,
        closure: &[LiveProcessIdentity],
        authority: &ExactLineageContainmentAuthority,
    ) -> bool {
        #[cfg(target_os = "macos")]
        {
            let publication_is_current = self.publication_fence.is_some_and(|epoch| {
                epoch == self.reconciliation_epoch && epoch > authority.inventory_epoch
            });
            if !publication_is_current
                || self.pending_reconciliation
                || self.reconciliation_requires_followup
            {
                return false;
            }
            let Some(gate) = self.gate.as_mut() else {
                return false;
            };
            if !gate.acknowledge_exact_containment(
                closure,
                &authority.allowed_fork_versions,
                authority.fork_event_version,
            ) {
                return false;
            }
            self.pending_reconciliation = true;
            self.publication_fence = None;
            true
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (closure, authority);
            false
        }
    }
}

#[cfg(all(test, target_os = "macos"))]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/process/process_events_exact_tests.rs"]
mod tests;
