use super::{
    CapturedProcessTree, FreezingProcessTree, FrozenProcessTree, KillPendingProcessTree,
    TerminationUnsupportedProcessTree, frozen_discovery::FrozenForkCoverage,
};
use crate::process::inventory_snapshot::{InventoryEntry, TestInventoryGuard};
use crate::process::{DescendantObservation, ProcessIdentity};
#[derive(Default)]
pub(super) struct CapturedTreeTestState {
    repeating_inventory: Option<Vec<InventoryEntry>>,
    capability_coverage_complete: bool,
    publication_quiescent: bool,
    frozen_fork_coverage_override: Option<FrozenForkCoverage>,
    kernel_eof_closure_stable_override: Option<bool>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapturedProcessTreeStateSnapshot {
    pub root: ProcessIdentity,
    pub root_exit: String,
    pub discovery_epoch: String,
    pub discovery_coverage_complete: bool,
    pub capability_scan_pending: bool,
    pub capability_scheduling_complete: bool,
    pub capability_coverage_complete: bool,
    pub publication_quiescent: bool,
    pub lineage_settled: bool,
    pub lineage_integrity_lost: bool,
    pub event_reconciliation_pending: bool,
    pub event_gate_present: bool,
    pub unresolved_fork_lineages: Vec<u64>,
    pub frozen_fork_reconciliation_required: bool,
    pub captured: Vec<ProcessIdentity>,
    pub captured_execution_states: Vec<String>,
    pub captured_lineages: Vec<u64>,
    pub retiring_lineages: Vec<u64>,
    pub exact_released_closure_covered: bool,
    pub capability_liveness: Option<String>,
    pub released_matched_lineages: Vec<u64>,
    pub released_pending_lineages: Vec<u64>,
    pub released_round_size: usize,
    pub released_sweep_diagnostic: Option<String>,
    pub released_holder_query_status: Option<String>,
    pub released_holder_pid_count: usize,
    pub released_holder_identity_count: usize,
    pub pty_master_released: bool,
    #[cfg(target_os = "macos")]
    pub transferable_descriptor: bool,
    pub descendant_observation: DescendantObservation,
}

impl CapturedTreeTestState {
    pub(super) fn install_inventory(&self) -> Option<TestInventoryGuard> {
        self.repeating_inventory
            .as_ref()
            .cloned()
            .map(crate::process::inventory_snapshot::install_repeating_test_inventory)
    }

    pub(super) fn observe_coverage(&mut self, coverage: FrozenForkCoverage) {
        self.capability_coverage_complete = coverage.capability_coverage_complete;
        self.publication_quiescent = coverage.publication_quiescent;
    }

    pub(super) fn take_frozen_fork_coverage_override(&mut self) -> Option<FrozenForkCoverage> {
        self.frozen_fork_coverage_override.take()
    }

    pub(super) fn kernel_eof_closure_stable_override(&self) -> Option<bool> {
        self.kernel_eof_closure_stable_override
    }

    pub(super) fn force_kernel_eof_closure_stable(&mut self) {
        self.kernel_eof_closure_stable_override = Some(true);
    }

    fn set_frozen_fork_coverage_override(&mut self, coverage: FrozenForkCoverage) {
        self.frozen_fork_coverage_override = Some(coverage);
    }
}

impl CapturedProcessTree {
    pub fn bind_repeating_process_inventory_for_test(
        &mut self,
        entries: &[(ProcessIdentity, u32, u32)],
    ) {
        self.test_state.repeating_inventory = Some(
            entries
                .iter()
                .map(|(identity, parent_pid, session_id)| {
                    crate::process::inventory_test_support::entry(
                        *identity,
                        *parent_pid,
                        *session_id,
                    )
                })
                .collect(),
        );
    }

    pub fn state_snapshot_for_test(&self) -> CapturedProcessTreeStateSnapshot {
        let targets = self.captured_targets();
        let live_targets = targets
            .iter()
            .map(|(live, _durable)| live.clone())
            .collect::<Vec<_>>();
        let captured_execution_states = live_targets
            .iter()
            .map(|identity| format!("{:?}", identity.execution_state()))
            .collect();
        let mut retiring_lineages = self
            .retiring
            .iter()
            .map(crate::process::LiveProcessIdentity::lineage_id)
            .collect::<Vec<_>>();
        retiring_lineages.sort_unstable();
        let (
            released_holder_query_status,
            released_holder_pid_count,
            released_holder_identity_count,
        ) = self.pty_scan.released_holder_observation_for_test();
        CapturedProcessTreeStateSnapshot {
            root: self.root(),
            root_exit: format!("{:?}", self.root_exit),
            discovery_epoch: format!("{:?}", self.discovery_epoch),
            discovery_coverage_complete: self.discovery_epoch.coverage_is_complete(),
            capability_scan_pending: self.capability_scan_pending,
            capability_scheduling_complete: self.capability_scheduling_complete,
            capability_coverage_complete: self.test_state.capability_coverage_complete,
            publication_quiescent: self.test_state.publication_quiescent,
            lineage_settled: self.lineage_settled,
            lineage_integrity_lost: self.lineage_integrity_lost,
            event_reconciliation_pending: self.events.reconciliation_pending_for_test(),
            event_gate_present: self.events.is_gated(),
            unresolved_fork_lineages: self.events.unresolved_fork_lineages_for_test(),
            frozen_fork_reconciliation_required: self.frozen_fork_reconciliation_required,
            captured: targets.iter().map(|(_live, durable)| *durable).collect(),
            captured_execution_states,
            captured_lineages: live_targets.iter().map(|live| live.lineage_id()).collect(),
            retiring_lineages,
            exact_released_closure_covered: self
                .pty_scan
                .covers_exact_released_closure(&live_targets),
            capability_liveness: self.pty_scan.liveness_for_test(),
            released_matched_lineages: self.pty_scan.released_matched_lineages_for_test(),
            released_pending_lineages: self.pty_scan.released_pending_lineages_for_test(),
            released_round_size: self.pty_scan.released_round_size_for_test(),
            released_sweep_diagnostic: self.pty_scan.released_sweep_diagnostic_for_test(),
            released_holder_query_status,
            released_holder_pid_count,
            released_holder_identity_count,
            pty_master_released: self.pty_master_released,
            #[cfg(target_os = "macos")]
            transferable_descriptor: self
                .pty_proof
                .as_ref()
                .is_some_and(super::PtySessionProof::has_transferable_descriptor),
            descendant_observation: self.descendant_observation(),
        }
    }

    pub fn authorize_destructive_for_test(&mut self) {
        self.destructive.authorize_for_test();
    }

    pub fn deny_destructive_for_test(&mut self) {
        self.destructive.deny_for_test();
    }

    pub fn force_next_frozen_fork_lineage_gap_for_test(&mut self) {
        self.test_state
            .set_frozen_fork_coverage_override(FrozenForkCoverage::MISSING_LINEAGE);
    }

    pub fn force_next_frozen_fork_capability_gap_for_test(&mut self) {
        self.test_state
            .set_frozen_fork_coverage_override(FrozenForkCoverage::MISSING_CAPABILITY);
    }

    pub fn force_next_frozen_fork_publication_gap_for_test(&mut self) {
        self.test_state
            .set_frozen_fork_coverage_override(FrozenForkCoverage::MISSING_PUBLICATION);
    }
}

macro_rules! delegate_snapshot {
    ($state:ty) => {
        impl $state {
            pub fn state_snapshot_for_test(&self) -> CapturedProcessTreeStateSnapshot {
                self.tree.state_snapshot_for_test()
            }
        }
    };
}

delegate_snapshot!(FreezingProcessTree);
delegate_snapshot!(FrozenProcessTree);
delegate_snapshot!(KillPendingProcessTree);
delegate_snapshot!(TerminationUnsupportedProcessTree);
