use super::{PROCESS_DETAIL, RecoverySnapshot};
use crate::host::{Host, RuntimeRecord};
use crate::pty_custody::{
    CustodyDurableLease, CustodyDurableRecord, CustodyGuardianBinding, CustodyHolderObservation,
    CustodyRuntimeIdentity,
};
use anyhow::{Result, ensure};

const RECEIPT_DETAIL: &str = "durable PTY custody retirement receipt is not exact";

pub(super) struct RetirementProof {
    guardian: CustodyGuardianBinding,
}

impl RetirementProof {
    pub(super) fn capture(host: &Host, snapshot: &RecoverySnapshot) -> Result<Option<Self>> {
        let lease = snapshot.validate_receipt(host)?;
        let guardian = exact_guardian(&snapshot.custody.runtime, lease)?;
        match guardian_observation(guardian) {
            CustodyHolderObservation::Current => Ok(None),
            CustodyHolderObservation::Indeterminate => anyhow::bail!(PROCESS_DETAIL),
            CustodyHolderObservation::ProvenGone => Ok(Some(Self { guardian })),
        }
    }

    pub(super) fn verify(&self, host: &Host, snapshot: &RecoverySnapshot) -> Result<()> {
        let lease = snapshot.validate_receipt(host)?;
        ensure!(
            exact_guardian(&snapshot.custody.runtime, lease)? == self.guardian
                && guardian_observation(self.guardian) == CustodyHolderObservation::ProvenGone,
            PROCESS_DETAIL
        );
        Ok(())
    }
}

fn exact_guardian(
    expected: &CustodyRuntimeIdentity,
    lease: CustodyDurableLease,
) -> Result<CustodyGuardianBinding> {
    let guardian = lease
        .guardian
        .ok_or_else(|| anyhow::anyhow!(RECEIPT_DETAIL))?;
    ensure!(
        guardian.matches_durable_lease(expected, &lease),
        RECEIPT_DETAIL
    );
    Ok(guardian)
}

pub(super) fn settlement_is_pending(host: &Host, expected: &CustodyRuntimeIdentity) -> bool {
    pending_binding(host, expected)
        .is_some_and(|binding| guardian_observation(binding) == CustodyHolderObservation::Current)
}

fn pending_binding(
    host: &Host,
    expected: &CustodyRuntimeIdentity,
) -> Option<CustodyGuardianBinding> {
    let runtime_path = host.paths.runtime_record(expected.terminal_id);
    let (runtime, _identity): (RuntimeRecord, crate::paths::identity::FileIdentity) =
        host.namespace.read_json_with_identity(&runtime_path).ok()?;
    let custody: CustodyDurableRecord = host
        .namespace
        .read_json(&host.paths.pty_custody(expected.terminal_id))
        .ok()?;
    let lease = custody
        .prepared_guardian_candidate()
        .or_else(|| custody.predecessor_lease())?;
    let guardian = lease.guardian?;
    (expected.matches_record(&runtime)
        && custody.is_valid()
        && custody.runtime == *expected
        && runtime.controller_generation <= lease.controller_generation
        && host.controller_generation >= lease.controller_generation
        && guardian.matches_durable_lease(expected, &lease))
    .then_some(guardian)
}

fn guardian_observation(guardian: CustodyGuardianBinding) -> CustodyHolderObservation {
    #[cfg(test)]
    if let Some(observation) = super::recovery_evidence_test_hook::take_guardian_observation() {
        return observation;
    }
    guardian.process().observation()
}
