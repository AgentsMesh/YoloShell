use super::*;
use crate::process::{ProcessIdentity, ProcessIdentityObservation};
use crate::pty_custody::CustodyRuntimeIdentity;
use anyhow::{Result, ensure};

#[cfg(target_os = "macos")]
#[path = "secondary_custody_recovery_macos.rs"]
mod platform;
#[cfg(target_os = "linux")]
use crate::host::linux_retirement::receipt as platform;
#[path = "secondary_custody_recovery_receipt.rs"]
mod receipt;
#[path = "secondary_custody_recovery_snapshot.rs"]
mod snapshot;
pub(in crate::host) use snapshot::RecoverySnapshot;

const PROCESS_DETAIL: &str = "durable PTY custody retirement process evidence changed";

pub(super) enum DurableRetirementObservation {
    Retired(DurableRetirementEvidence),
    AwaitingSettlement,
    Unavailable,
}

pub(super) struct DurableRetirementEvidence {
    snapshot: RecoverySnapshot,
    platform: platform::RetirementProof,
}

impl DurableRetirementEvidence {
    pub(super) fn capture(
        host: &Host,
        expected: &CustodyRuntimeIdentity,
    ) -> DurableRetirementObservation {
        Self::try_capture(host, expected).unwrap_or(DurableRetirementObservation::Unavailable)
    }

    fn try_capture(
        host: &Host,
        expected: &CustodyRuntimeIdentity,
    ) -> Result<DurableRetirementObservation> {
        let snapshot = RecoverySnapshot::capture(host, expected)?;
        snapshot.validate_receipt(host)?;
        require_gone_runtime_processes(&snapshot.runtime)?;
        let Some(platform) = platform::RetirementProof::capture(host, &snapshot)? else {
            return Ok(DurableRetirementObservation::AwaitingSettlement);
        };
        let evidence = Self { snapshot, platform };
        evidence.verify(host)?;
        Ok(DurableRetirementObservation::Retired(evidence))
    }

    pub(super) fn verify(&self, host: &Host) -> Result<()> {
        self.snapshot.verify(host)?;
        self.snapshot.validate_receipt(host)?;
        require_gone_runtime_processes(&self.snapshot.runtime)?;
        self.platform.verify(host, &self.snapshot)?;
        self.snapshot.journal.verify_current().map_err(Into::into)
    }
}

pub(super) fn settlement_is_pending(host: &Host, expected: &CustodyRuntimeIdentity) -> bool {
    platform::settlement_is_pending(host, expected)
}

pub(in crate::host) fn require_gone_runtime_processes(runtime: &RuntimeRecord) -> Result<()> {
    ensure!(
        [
            ProcessIdentity {
                pid: runtime.worker_pid,
                start_time: runtime.worker_start_time
            },
            ProcessIdentity {
                pid: runtime.child_pid,
                start_time: runtime.child_start_time
            },
        ]
        .into_iter()
        .all(|process| process.observe() == ProcessIdentityObservation::Gone),
        PROCESS_DETAIL
    );
    Ok(())
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../../test_support/unit/secondary_custody/secondary_custody_recovery_evidence_test_hook.rs"]
mod recovery_evidence_test_hook;

#[cfg(test)]
pub(super) use recovery_evidence_test_hook::install_guardian_observations_for_test;
