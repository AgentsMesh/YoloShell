use super::snapshot::{ContainmentSnapshot, root};
use crate::host::{Host, secondary_custody::RecoverySnapshot};
use crate::process::linux_containment::LinuxContainmentSettlement;
use crate::pty_custody::{CustodyDescriptorSet, CustodyDurableLease, CustodyRuntimeIdentity};
use anyhow::{Result, ensure};

const RECEIPT_DETAIL: &str = "Linux retirement receipt does not identify its exact custody lease";

pub(in crate::host) struct RetirementProof {
    containment: ContainmentSnapshot,
    settlement: LinuxContainmentSettlement,
}

impl RetirementProof {
    pub(in crate::host) fn capture(
        host: &Host,
        snapshot: &RecoverySnapshot,
    ) -> Result<Option<Self>> {
        validate_lease(host, snapshot)?;
        let runtime = &snapshot.runtime;
        let containment = ContainmentSnapshot::capture(host, runtime)?;
        let Some(settlement) = LinuxContainmentSettlement::observe_settled(
            containment.record.clone(),
            runtime.terminal_id,
            runtime.runtime_epoch,
            root(runtime),
        )?
        else {
            return Ok(None);
        };
        let proof = Self::from_settlement(containment, settlement);
        proof.verify(host, snapshot)?;
        Ok(Some(proof))
    }

    pub(super) fn from_settlement(
        containment: ContainmentSnapshot,
        settlement: LinuxContainmentSettlement,
    ) -> Self {
        Self {
            containment,
            settlement,
        }
    }

    pub(in crate::host) fn verify(&self, host: &Host, snapshot: &RecoverySnapshot) -> Result<()> {
        validate_lease(host, snapshot)?;
        self.containment.verify(host, &snapshot.runtime)?;
        ensure!(
            self.settlement.record() == &self.containment.record,
            RECEIPT_DETAIL
        );
        self.settlement.verify()?;
        self.containment.verify(host, &snapshot.runtime)
    }
}

pub(super) fn validate_lease(
    host: &Host,
    snapshot: &RecoverySnapshot,
) -> Result<CustodyDurableLease> {
    let lease = snapshot.validate_lease(host)?;
    ensure!(
        lease.descriptor_set == CustodyDescriptorSet::LinuxContainment
            && lease.profile.is_active()
            && lease.guardian.is_none(),
        RECEIPT_DETAIL
    );
    // Preserve the exact committed lease, including its opaque composite identity.
    // Independent cgroup authority does not reconstruct a released PTY master.
    Ok(lease)
}

pub(in crate::host) fn settlement_is_pending(
    host: &Host,
    expected: &CustodyRuntimeIdentity,
) -> bool {
    let Ok(snapshot) = RecoverySnapshot::capture(host, expected) else {
        return false;
    };
    matches!(RetirementProof::capture(host, &snapshot), Ok(None))
}
