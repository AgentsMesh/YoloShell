use crate::process::{ProcessIdentity, RetainedExactProcess, RetainedProcessObservation};
use crate::pty_custody::CustodyRuntimeIdentity;
use anyhow::Result;
use tokio::time::Instant;

pub(super) const NAMESPACE_LOSS_OBSERVATION_GRACE: std::time::Duration =
    crate::session_timing::GRACEFUL_TERMINATION_TIMEOUT;
pub(super) const NAMESPACE_LOSS_GRACEFUL_GRACE: std::time::Duration =
    crate::session_timing::GRACEFUL_TERMINATION_TIMEOUT;
const TARGET_RECONCILIATION_CONTEXT: &str = "reconcile exact namespace-loss containment target";

pub(super) struct NamespaceLossContainment {
    runtime: CustodyRuntimeIdentity,
    authority: Option<crate::process::MacosNamespaceLossContainmentAuthority>,
    worker: ExactTarget,
    child: ExactTarget,
    holders: Vec<ExactTarget>,
    phase: Phase,
    finalized: Option<NamespaceLossExactSettlement>,
    late_discovery: discovery::LateHolderDiscovery,
    #[cfg(test)]
    holder_discovery_scans: usize,
}

#[derive(Clone)]
pub(super) struct NamespaceLossExactSettlement {
    finalized: crate::process::MacosNamespaceLossFinalized,
}

enum Phase {
    Observing { deadline: Instant },
    Graceful { deadline: Instant },
    Force,
}

impl NamespaceLossContainment {
    pub(super) fn new(runtime: &CustodyRuntimeIdentity, now: Instant) -> Self {
        Self {
            runtime: runtime.clone(),
            authority: None,
            worker: ExactTarget::new(ProcessIdentity {
                pid: runtime.worker_pid,
                start_time: runtime.worker_start_time,
            }),
            child: ExactTarget::new(ProcessIdentity {
                pid: runtime.child_pid,
                start_time: runtime.child_start_time,
            }),
            holders: Vec::new(),
            phase: Phase::Observing {
                deadline: now + NAMESPACE_LOSS_OBSERVATION_GRACE,
            },
            finalized: None,
            late_discovery: discovery::LateHolderDiscovery::default(),
            #[cfg(test)]
            holder_discovery_scans: usize::default(),
        }
    }

    fn reconcile(&mut self, now: Instant) -> Result<bool> {
        match self.phase {
            Phase::Observing { deadline } if now < deadline => self.apply(ExactTarget::capture),
            Phase::Observing { .. } => {
                self.apply(ExactTarget::begin_graceful)?;
                self.phase = Phase::Graceful {
                    deadline: now + NAMESPACE_LOSS_GRACEFUL_GRACE,
                };
                Ok(())
            }
            Phase::Graceful { deadline } if now < deadline => {
                self.apply(ExactTarget::begin_graceful)
            }
            Phase::Graceful { .. } => {
                self.apply(ExactTarget::enforce_force)?;
                self.phase = Phase::Force;
                Ok(())
            }
            Phase::Force => self.apply(ExactTarget::enforce_force),
        }?;
        self.exact_settlement()
    }

    #[cfg(test)]
    pub(super) fn reconcile_for_test(&mut self, now: Instant) -> Result<bool> {
        self.reconcile(now)
    }

    fn apply(&mut self, operation: fn(&mut ExactTarget) -> Result<()>) -> Result<()> {
        let mut outcome = Ok(());
        holders::retain_error(&mut outcome, operation(&mut self.worker));
        holders::retain_error(&mut outcome, operation(&mut self.child));
        for holder in &mut self.holders {
            holders::retain_error(&mut outcome, operation(holder));
        }
        outcome
    }

    #[cfg(test)]
    pub(super) fn force_requested(&self) -> bool {
        matches!(self.phase, Phase::Force)
    }
}

pub(super) struct ExactTarget {
    pub(super) identity: ProcessIdentity,
    authority: Option<RetainedExactProcess>,
    capture_complete: bool,
    graceful_complete: bool,
}

impl ExactTarget {
    pub(super) fn new(identity: ProcessIdentity) -> Self {
        Self {
            identity,
            authority: None,
            capture_complete: false,
            graceful_complete: false,
        }
    }

    pub(super) fn capture(&mut self) -> Result<()> {
        if !self.capture_complete {
            self.authority = RetainedExactProcess::capture(self.identity)?;
            self.capture_complete = true;
        }
        Ok(())
    }

    pub(super) fn begin_graceful(&mut self) -> Result<()> {
        if self.graceful_complete {
            return Ok(());
        }
        self.capture()?;
        if let Some(authority) = &mut self.authority {
            let _ = RetainedExactProcess::begin_containment(authority)?;
        }
        self.graceful_complete = true;
        Ok(())
    }

    pub(super) fn enforce_force(&mut self) -> Result<()> {
        self.capture()?;
        if let Some(authority) = &mut self.authority {
            let _ = RetainedExactProcess::enforce_containment(authority)?;
        }
        Ok(())
    }

    pub(super) fn observe_settled(&mut self) -> Result<bool> {
        self.capture()?;
        let Some(authority) = &mut self.authority else {
            return Ok(true);
        };
        Ok(RetainedExactProcess::observe(authority)? == RetainedProcessObservation::Settled)
    }

    #[cfg(test)]
    pub(super) fn fail_next_refresh_for_test(&mut self, detail: &'static str) {
        self.authority
            .as_mut()
            .expect(TARGET_RECONCILIATION_CONTEXT)
            .fail_next_refresh_for_test(detail);
    }
}

impl NamespaceLossExactSettlement {
    fn new(finalized: crate::process::MacosNamespaceLossFinalized) -> Self {
        Self { finalized }
    }

    pub(super) fn finalized(&self) -> &crate::process::MacosNamespaceLossFinalized {
        &self.finalized
    }
}

#[path = "settlement_containment_discovery.rs"]
mod discovery;
#[path = "settlement_containment_holders.rs"]
mod holders;

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
include!("../unit_tests/settlement_containment_test_wiring.rs");
