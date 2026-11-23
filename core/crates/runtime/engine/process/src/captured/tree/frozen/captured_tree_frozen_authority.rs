use super::CapturedProcessTree;
use crate::process::live_identity::ProcessExecutionState;
use crate::process::{DescendantObservation, LiveProcessIdentity, ProcessIdentity};

pub(super) struct CapturedStoppedClosureAuthority {
    closure: Vec<LiveProcessIdentity>,
    fork_sources: Vec<LiveProcessIdentity>,
}

#[derive(Clone, Copy)]
pub(super) enum FrozenClosureAuthority<'a> {
    CapturedStopped(&'a CapturedStoppedClosureAuthority),
    OutputFencedEmpty,
}

impl CapturedProcessTree {
    pub(super) fn frozen_closure_is_settled(
        &mut self,
        authority: FrozenClosureAuthority<'_>,
    ) -> bool {
        match authority {
            FrozenClosureAuthority::CapturedStopped(authority)
                if !self.captured_stopped_closure_can_freeze(authority) =>
            {
                return false;
            }
            FrozenClosureAuthority::OutputFencedEmpty
                if self.frozen_fork_reconciliation_required =>
            {
                return false;
            }
            _ => {}
        }
        match self.descendant_observation() {
            DescendantObservation::Settled => match authority {
                FrozenClosureAuthority::CapturedStopped(_) => true,
                FrozenClosureAuthority::OutputFencedEmpty => {
                    self.empty_output_fence_event_authority_is_current()
                }
            },
            DescendantObservation::CapabilityScanPending => match authority {
                FrozenClosureAuthority::CapturedStopped(_) => true,
                FrozenClosureAuthority::OutputFencedEmpty => {
                    self.empty_output_fence_event_authority_is_current()
                        && self.empty_output_fence_is_authorized()
                }
            },
            DescendantObservation::Indeterminate => {
                matches!(authority, FrozenClosureAuthority::OutputFencedEmpty)
                    && self.empty_output_fence_can_reconcile_uncertainty()
            }
        }
    }

    pub(super) fn captured_stopped_closure_authority(
        &self,
        targets: &[(LiveProcessIdentity, ProcessIdentity)],
    ) -> anyhow::Result<Option<CapturedStoppedClosureAuthority>> {
        self.require_destructive_session()?;
        if targets.is_empty() || self.captured_targets() != targets {
            return Ok(None);
        }
        let mut closure = Vec::with_capacity(targets.len());
        let mut fork_sources = Vec::with_capacity(targets.len());
        for (identity, _durable) in targets {
            match identity.execution_state()? {
                ProcessExecutionState::Running => {
                    #[cfg(all(
                        feature = "adversarial-test-hooks",
                        not(test),
                        target_os = "macos"
                    ))]
                    self.trace_running_freeze_target(identity);
                    return Ok(None);
                }
                ProcessExecutionState::Stopped => {
                    fork_sources.push(identity.clone());
                }
                ProcessExecutionState::Gone => {}
            }
            closure.push(identity.clone());
        }
        Ok(Some(CapturedStoppedClosureAuthority {
            closure,
            fork_sources,
        }))
    }

    fn captured_stopped_closure_can_freeze(
        &self,
        authority: &CapturedStoppedClosureAuthority,
    ) -> bool {
        !self.frozen_fork_reconciliation_required
            && !self.events.has_unresolved_fork()
            && self
                .captured_targets()
                .iter()
                .map(|(identity, _durable)| identity)
                .eq(&authority.closure)
    }

    pub(super) fn empty_output_fence_is_authorized(&self) -> bool {
        if self.pty_master_released
            || self
                .pty_proof
                .as_ref()
                .is_none_or(super::PtySessionProof::is_process_discovery)
        {
            return false;
        }
        self.pty_scan.proves_root_exit() || self.premature_release_empty_closure_coverage()
    }

    #[cfg(all(test, target_os = "macos"))]
    pub(super) fn positive_closure_coverage(&self, closure: &[LiveProcessIdentity]) -> bool {
        !self.frozen_fork_reconciliation_required
            && self.frozen_fork_acknowledgement_coverage(closure)
    }

    #[cfg(target_os = "macos")]
    pub(super) fn frozen_fork_acknowledgement_coverage(
        &self,
        closure: &[LiveProcessIdentity],
    ) -> bool {
        self.pending_capability_coverage()
            || self.released_scheduling_coverage()
            || (self.pty_scan.proves_root_exit()
                && self
                    .pty_proof
                    .as_ref()
                    .is_some_and(super::PtySessionProof::has_transferable_descriptor)
                && self.pty_scan.covers_exact_released_closure(closure))
    }

    #[cfg(target_os = "macos")]
    pub(super) fn released_scheduling_coverage(&self) -> bool {
        self.capability_scheduling_complete
            && self.pty_scan.proves_root_exit()
            && self
                .pty_proof
                .as_ref()
                .is_some_and(super::PtySessionProof::has_transferable_descriptor)
    }

    #[cfg(not(target_os = "macos"))]
    pub(super) fn frozen_fork_acknowledgement_coverage(
        &self,
        _closure: &[LiveProcessIdentity],
    ) -> bool {
        false
    }

    #[cfg(not(target_os = "macos"))]
    pub(super) fn released_scheduling_coverage(&self) -> bool {
        false
    }
}

impl CapturedStoppedClosureAuthority {
    pub(super) fn all_members_stopped(&self) -> bool {
        self.fork_sources.len() == self.closure.len()
    }

    pub(super) fn closure(&self) -> &[LiveProcessIdentity] {
        &self.closure
    }

    pub(super) fn fork_sources(&self) -> &[LiveProcessIdentity] {
        &self.fork_sources
    }

    pub(super) fn into_closure(self) -> Vec<LiveProcessIdentity> {
        self.closure
    }
}

include!("captured_tree_frozen_authority_coverage.rs");
