use super::LiveProcessIdentity;
use exact::ProcessEventInventoryEpoch;
use std::time::{Duration, Instant};

#[cfg(target_os = "macos")]
use super::process_events_macos::MacosProcessEvents;

const PROCESS_EVENT_AUDIT_INTERVAL: Duration = Duration::from_secs(30);
const EVENT_GATE_FAILURE_LOG: &str =
    "process event gate failed; reverting to dense process inventory polling";

pub(super) struct ProcessEvents {
    #[cfg(target_os = "macos")]
    gate: Option<MacosProcessEvents>,
    reconciled_at: Instant,
    pending_reconciliation: bool,
    reconciliation_requires_followup: bool,
    reconciliation_epoch: ProcessEventInventoryEpoch,
    publication_fence: Option<ProcessEventInventoryEpoch>,
}

impl ProcessEvents {
    pub(super) fn new(root: LiveProcessIdentity) -> Self {
        #[cfg(target_os = "macos")]
        let gate = MacosProcessEvents::new(root).map_err(report_failure).ok();
        #[cfg(not(target_os = "macos"))]
        let _ = root;
        Self {
            #[cfg(target_os = "macos")]
            gate,
            reconciled_at: Instant::now(),
            pending_reconciliation: false,
            reconciliation_requires_followup: false,
            reconciliation_epoch: ProcessEventInventoryEpoch::default(),
            publication_fence: None,
        }
    }

    pub(super) fn new_in_spawn_boundary(
        root: LiveProcessIdentity,
        boundary: &super::WaitableChildSpawnBoundary<'_>,
    ) -> Self {
        #[cfg(target_os = "macos")]
        let gate = MacosProcessEvents::new_in_spawn_boundary(root, boundary)
            .map_err(report_failure)
            .ok();
        #[cfg(not(target_os = "macos"))]
        let _ = (root, boundary);
        Self {
            #[cfg(target_os = "macos")]
            gate,
            reconciled_at: Instant::now(),
            pending_reconciliation: false,
            reconciliation_requires_followup: false,
            reconciliation_epoch: ProcessEventInventoryEpoch::default(),
            publication_fence: None,
        }
    }

    pub(super) fn requires_reconciliation(&mut self) -> bool {
        self.invalidate_inventory_publication();
        let mut required = self.pending_reconciliation || self.audit_is_due();
        #[cfg(target_os = "macos")]
        {
            let Some(gate) = self.gate.as_mut() else {
                self.pending_reconciliation = true;
                return true;
            };
            match gate.changed() {
                Ok(changed) => {
                    self.pending_reconciliation |= changed;
                    required |= changed;
                    required
                }
                Err(error) => {
                    report_failure(error);
                    self.gate = None;
                    self.pending_reconciliation = true;
                    self.reconciliation_requires_followup = true;
                    true
                }
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = required;
            true
        }
    }

    fn audit_is_due(&self) -> bool {
        self.reconciled_at.elapsed() >= PROCESS_EVENT_AUDIT_INTERVAL
    }

    pub(super) fn lineage_uncertain(&self) -> bool {
        #[cfg(target_os = "macos")]
        {
            self.gate
                .as_ref()
                .is_none_or(MacosProcessEvents::has_unresolved_fork)
        }
        #[cfg(not(target_os = "macos"))]
        {
            false
        }
    }
    pub(super) fn has_unresolved_fork(&self) -> bool {
        #[cfg(target_os = "macos")]
        return self
            .gate
            .as_ref()
            .is_some_and(MacosProcessEvents::has_unresolved_fork);
        #[cfg(not(target_os = "macos"))]
        false
    }

    #[cfg(target_os = "macos")]
    pub(super) fn exact_closure_isolated(&self, closure: &[LiveProcessIdentity]) -> bool {
        self.gate
            .as_ref()
            .is_some_and(|gate| gate.exact_closure_isolated(closure))
    }

    #[cfg(not(target_os = "macos"))]
    pub(super) fn exact_closure_isolated(&self, _closure: &[LiveProcessIdentity]) -> bool {
        false
    }

    pub(super) fn acknowledge_frozen_forks(
        &mut self,
        frozen_lineages: &std::collections::HashSet<u64>,
    ) -> bool {
        self.invalidate_inventory_publication();
        #[cfg(target_os = "macos")]
        return self
            .gate
            .as_mut()
            .is_some_and(|gate| gate.acknowledge_frozen_forks(frozen_lineages));
        #[cfg(not(target_os = "macos"))]
        {
            let _ = frozen_lineages;
            false
        }
    }

    pub(super) fn acknowledge_root_exit_discovery_epoch(&mut self) {
        self.invalidate_inventory_publication();
        self.pending_reconciliation = true;
        #[cfg(target_os = "macos")]
        if let Some(gate) = self.gate.as_mut() {
            gate.acknowledge_root_exit_discovery_epoch();
        }
    }

    pub(in crate::process) fn confirm_root_exit(&mut self, identity: LiveProcessIdentity) {
        #[cfg(target_os = "macos")]
        if let Some(gate) = self.gate.as_mut() {
            gate.confirm_root_exit(identity);
        }
        #[cfg(not(target_os = "macos"))]
        let _ = identity;
    }

    pub(super) fn is_gated(&self) -> bool {
        #[cfg(target_os = "macos")]
        {
            self.gate.is_some()
        }
        #[cfg(not(target_os = "macos"))]
        {
            false
        }
    }
}

#[path = "process_events_exact.rs"]
mod exact;
pub(super) use exact::ExactLineageContainmentAuthority;

#[path = "process_events_reconcile.rs"]
mod reconcile;

#[cfg(target_os = "macos")]
fn report_failure(error: anyhow::Error) {
    tracing::warn!(%error, EVENT_GATE_FAILURE_LOG);
}

#[cfg(any(test, feature = "runtime-component-tests"))]
#[path = "../../test_support/process_events/process_events.rs"]
mod test_support;

#[cfg(all(test, target_os = "macos", feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/process/process_events_frozen_acknowledgement_tests.rs"]
mod frozen_acknowledgement_tests;
#[cfg(all(test, target_os = "macos", feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/process/process_events_reconciliation_tests.rs"]
mod reconciliation_tests;
