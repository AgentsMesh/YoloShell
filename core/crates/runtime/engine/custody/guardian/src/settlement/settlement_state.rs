use crate::model::RuntimeRecord;
use crate::paths::identity::FileIdentity;
use crate::pty_custody::{
    CustodyDurableRecord, CustodyDurableSnapshot, CustodyGuardianBinding,
    CustodyGuardianPreparation, CustodyHolderIdentity, CustodyHolderObservation,
    CustodyRuntimeIdentity,
};
use anyhow::{Result, ensure};

use super::settlement::{PROCESS_DETAIL, RECORD_DETAIL};
use super::settlement_runtime_file::RetainedRuntimeFile;

#[path = "settlement_state_pending.rs"]
mod pending;
#[path = "settlement_state_runtime.rs"]
mod runtime;
use runtime::RetainedRuntime;

pub(super) struct GuardianState {
    binding: CustodyGuardianBinding,
    runtime: CustodyRuntimeIdentity,
    retained_runtime: RetainedRuntime,
    pending: Option<CustodyGuardianPreparation>,
    pending_record: Option<CustodyDurableRecord>,
    pending_predecessor: Option<CustodyDurableSnapshot>,
    was_armed: bool,
    armed_promotion_verified: bool,
}

impl GuardianState {
    pub(super) fn new(
        preparation: CustodyGuardianPreparation,
        prepared: CustodyDurableRecord,
        predecessor: CustodyDurableSnapshot,
        runtime_record: RuntimeRecord,
        runtime_identity: FileIdentity,
        retained_runtime_file: RetainedRuntimeFile,
    ) -> Result<Self> {
        let runtime = CustodyRuntimeIdentity::from_record(&runtime_record);
        ensure!(runtime == prepared.runtime, RECORD_DETAIL);
        Ok(Self {
            runtime,
            retained_runtime: RetainedRuntime::Exact {
                record: runtime_record,
                identity: runtime_identity,
                _file: retained_runtime_file,
            },
            binding: preparation.binding(),
            pending: Some(preparation),
            pending_record: Some(prepared),
            pending_predecessor: Some(predecessor),
            was_armed: false,
            armed_promotion_verified: false,
        })
    }

    pub(super) fn was_armed(&self) -> bool {
        self.was_armed
    }

    pub(super) fn mark_armed(&mut self) {
        self.was_armed = true;
    }

    pub(super) fn needs_armed_promotion(&self) -> bool {
        !self.armed_promotion_verified
    }

    pub(super) fn mark_armed_promotion_verified(&mut self) {
        self.armed_promotion_verified = true;
    }

    pub(super) fn adopt_durable_binding(&mut self, record: &CustodyDurableRecord) -> Result<()> {
        let lease = record
            .predecessor_lease()
            .ok_or_else(|| anyhow::anyhow!(RECORD_DETAIL))?;
        if lease.guardian == Some(self.binding) {
            if self
                .pending
                .is_some_and(|pending| pending.binding() == self.binding)
            {
                self.clear_pending();
            }
            return exact_binding(lease.holder, lease.controller_generation, self.binding);
        }
        let pending = self.pending.ok_or_else(|| anyhow::anyhow!(RECORD_DETAIL))?;
        ensure!(lease.guardian == Some(pending.binding()), RECORD_DETAIL);
        exact_binding(lease.holder, lease.controller_generation, pending.binding())?;
        self.binding = pending.binding();
        self.clear_pending();
        Ok(())
    }

    pub(super) fn clear_abandoned_prepare(&mut self, record: &CustodyDurableRecord) -> Result<()> {
        let Some(pending) = self.pending else {
            return Ok(());
        };
        if record.predecessor_lease().and_then(|lease| lease.guardian) == Some(pending.binding()) {
            return Ok(());
        }
        match pending.binding().host_holder().observation() {
            CustodyHolderObservation::Current => {}
            CustodyHolderObservation::ProvenGone => self.clear_pending(),
            CustodyHolderObservation::Indeterminate => anyhow::bail!(PROCESS_DETAIL),
        }
        Ok(())
    }

    #[cfg(test)]
    pub(super) fn new_prepared_fixture(
        preparation: CustodyGuardianPreparation,
        prepared: CustodyDurableRecord,
        predecessor: CustodyDurableSnapshot,
        runtime: CustodyRuntimeIdentity,
    ) -> Self {
        Self {
            runtime,
            retained_runtime: RetainedRuntime::PreparedOnly,
            binding: preparation.binding(),
            pending: Some(preparation),
            pending_record: Some(prepared),
            pending_predecessor: Some(predecessor),
            was_armed: false,
            armed_promotion_verified: false,
        }
    }
}

fn exact_binding(
    holder: CustodyHolderIdentity,
    generation: u64,
    binding: CustodyGuardianBinding,
) -> Result<()> {
    ensure!(
        binding.host_holder() == holder && binding.controller_generation() == generation,
        RECORD_DETAIL
    );
    ensure!(binding.process().matches(), RECORD_DETAIL);
    Ok(())
}
