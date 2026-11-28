use super::RecoverySnapshot;
use crate::host::Host;
use crate::pty_custody::{CustodyDurableLease, CustodyRetirementEvidence};
use anyhow::{Result, ensure};

const RECEIPT_DETAIL: &str = "durable PTY custody retirement receipt is not exact";

impl RecoverySnapshot {
    pub(in crate::host) fn validate_lease(&self, host: &Host) -> Result<CustodyDurableLease> {
        ensure!(
            self.custody.is_valid()
                && self.custody.runtime.matches_record(&self.runtime)
                && crate::process::boot_identity_relation(&self.runtime.boot_id, &host.boot_id)
                    == crate::process::BootIdentityRelation::Same,
            RECEIPT_DETAIL
        );
        let lease = self
            .custody
            .armed_lease()
            .or_else(|| self.custody.retired_lease())
            .ok_or_else(|| anyhow::anyhow!(RECEIPT_DETAIL))?;
        ensure!(
            self.runtime.controller_generation == lease.controller_generation
                && host.controller_generation >= lease.controller_generation,
            RECEIPT_DETAIL
        );
        Ok(lease)
    }

    pub(in crate::host) fn evidence(
        &self,
        lease: CustodyDurableLease,
    ) -> CustodyRetirementEvidence {
        self.journal
            .custody_retirement_evidence(lease.controller_generation, self.runtime_identity)
    }

    pub(in crate::host) fn validate_receipt(&self, host: &Host) -> Result<CustodyDurableLease> {
        let lease = self.validate_lease(host)?;
        let evidence = self
            .custody
            .retirement_evidence()
            .ok_or_else(|| anyhow::anyhow!(RECEIPT_DETAIL))?;
        ensure!(
            self.custody.retired_lease() == Some(lease)
                && evidence.is_valid()
                && evidence == self.evidence(lease),
            RECEIPT_DETAIL
        );
        Ok(lease)
    }
}
