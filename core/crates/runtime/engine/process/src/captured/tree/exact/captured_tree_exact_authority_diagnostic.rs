use super::frozen_authority::CapturedStoppedClosureAuthority;
use super::{CapturedProcessTree, FreezingProcessTree, FrozenProcessTree, KillPendingProcessTree};
use crate::process::LiveProcessIdentity;

const EXACT_AUTHORITY_DIAGNOSTIC: &str = "adversarial exact PTY authority diagnostic";
const RUNNING_TARGET_DIAGNOSTIC: &str = "adversarial freeze target remains running";

#[derive(Debug)]
pub(super) enum ExactAuthorityLane {
    Global,
    Incremental,
}

impl CapturedProcessTree {
    #[cfg(any(test, feature = "adversarial-test-hooks"))]
    pub fn adversarial_authority_snapshot(&self) -> String {
        let targets = self.captured_targets();
        let mut unresolved_sources = self
            .events
            .unresolved_fork_versions()
            .into_keys()
            .collect::<Vec<_>>();
        unresolved_sources.sort_unstable();
        let closure = targets
            .iter()
            .map(|(identity, _durable)| {
                format!(
                    "{}:{}:{:?}",
                    identity.pid(),
                    identity.lineage_id(),
                    identity.execution_state()
                )
            })
            .collect::<Vec<_>>();
        let live = targets
            .iter()
            .map(|(identity, _durable)| identity.clone())
            .collect::<Vec<_>>();
        let transferable = self
            .pty_proof
            .as_ref()
            .is_some_and(super::PtySessionProof::has_transferable_descriptor);
        let bootstrap_confirmed = self
            .pty_proof
            .as_ref()
            .is_some_and(super::PtySessionProof::bootstrap_exec_was_confirmed);
        let lease_present = self
            .pty_proof
            .as_ref()
            .is_none_or(super::PtySessionProof::has_required_pty_object_lease);
        format!(
            "root={} root_exit_state={:?} closure={closure:?} unresolved_sources={unresolved_sources:?} discovery={} lineage_settled={} lineage_lost={} \
             event_gate={} unresolved_fork={} frozen_reconciliation={} capability_pending={} \
             scheduling_complete={} writer_coverage={} released_coverage={} transferable={} bootstrap_confirmed={} \
             lease={} destructive={} root_exit={} observation={:?} scan=[{}]",
            self.live_root().pid(),
            self.root_exit,
            self.discovery_epoch.coverage_is_complete(),
            self.lineage_settled,
            self.lineage_integrity_lost,
            self.events.is_gated(),
            self.events.has_unresolved_fork(),
            self.frozen_fork_reconciliation_required,
            self.capability_scan_pending,
            self.capability_scheduling_complete,
            self.writer_present_coverage(),
            self.pty_scan.covers_exact_released_closure(&live),
            transferable,
            bootstrap_confirmed,
            lease_present,
            self.destructive_authority().is_ok(),
            self.root_exit_is_proven(),
            self.descendant_observation(),
            self.pty_scan.adversarial_snapshot(),
        )
    }

    pub(super) fn trace_exact_authority(
        &self,
        lane: ExactAuthorityLane,
        authority: &CapturedStoppedClosureAuthority,
        discovery_covered: bool,
        isolated: bool,
        pty_authorized: bool,
    ) {
        let closure = process_ids(authority.closure());
        let captured = self
            .captured_targets()
            .iter()
            .map(|(identity, _durable)| identity.pid())
            .collect::<Vec<_>>();
        let retiring = self
            .retiring
            .iter()
            .map(LiveProcessIdentity::pid)
            .collect::<Vec<_>>();
        let transferable = self
            .pty_proof
            .as_ref()
            .is_some_and(super::PtySessionProof::has_transferable_descriptor);
        let lease_present = self
            .pty_proof
            .as_ref()
            .is_none_or(super::PtySessionProof::has_required_pty_object_lease);
        tracing::warn!(
            ?lane,
            root_pid = self.live_root().pid(),
            ?closure,
            ?captured,
            ?retiring,
            discovery_covered,
            isolated,
            pty_authorized,
            all_stopped = authority.all_members_stopped(),
            lineage_settled = self.lineage_settled,
            lineage_integrity_lost = self.lineage_integrity_lost,
            event_gate = self.events.is_gated(),
            unresolved_fork = self.events.has_unresolved_fork(),
            frozen_reconciliation = self.frozen_fork_reconciliation_required,
            capability_pending = self.capability_scan_pending,
            capability_scheduling_complete = self.capability_scheduling_complete,
            writer_coverage = self.writer_present_coverage(),
            released_coverage = self.pty_scan.covers_exact_released_closure(authority.closure()),
            transferable,
            lease_present,
            destructive_available = self.destructive_authority().is_ok(),
            root_exit_proven = self.root_exit_is_proven(),
            descendant_observation = ?self.descendant_observation(),
            message = EXACT_AUTHORITY_DIAGNOSTIC,
        );
    }

    pub(super) fn trace_running_freeze_target(&self, target: &LiveProcessIdentity) {
        let captured = self
            .captured_targets()
            .iter()
            .map(|(identity, _durable)| identity.pid())
            .collect::<Vec<_>>();
        tracing::warn!(
            root_pid = self.live_root().pid(),
            target_pid = target.pid(),
            ?captured,
            capability_pending = self.capability_scan_pending,
            discovery_covered = self.discovery_epoch.coverage_is_complete(),
            message = RUNNING_TARGET_DIAGNOSTIC,
        );
    }
}

impl FreezingProcessTree {
    #[cfg(any(test, feature = "adversarial-test-hooks"))]
    pub fn adversarial_authority_snapshot(&self) -> String {
        self.tree.adversarial_authority_snapshot()
    }
}

impl FrozenProcessTree {
    #[cfg(any(test, feature = "adversarial-test-hooks"))]
    pub fn adversarial_authority_snapshot(&self) -> String {
        self.tree.adversarial_authority_snapshot()
    }
}

impl KillPendingProcessTree {
    #[cfg(any(test, feature = "adversarial-test-hooks"))]
    pub fn adversarial_authority_snapshot(&self) -> String {
        self.tree.adversarial_authority_snapshot()
    }
}

fn process_ids(identities: &[LiveProcessIdentity]) -> Vec<u32> {
    identities.iter().map(LiveProcessIdentity::pid).collect()
}
