use super::GuardianState;
use crate::custody_guardian::settlement_runtime_file::RetainedRuntimeFile;
use crate::model::RuntimeRecord;
use crate::paths::identity::FileIdentity;
use crate::pty_custody::{
    CustodyDurableRecord, CustodyDurableSnapshot, CustodyGuardianBinding,
    CustodyGuardianPreparation,
};
use anyhow::{Result, ensure};

use super::super::settlement::RECORD_DETAIL;

impl GuardianState {
    pub(in crate::custody_guardian) fn binding(&self) -> CustodyGuardianBinding {
        self.binding
    }

    pub(in crate::custody_guardian) fn pending(&self) -> Option<CustodyGuardianPreparation> {
        self.pending
    }

    pub(in crate::custody_guardian) fn pending_record(&self) -> Option<&CustodyDurableRecord> {
        self.pending_record.as_ref()
    }

    pub(in crate::custody_guardian) fn pending_predecessor(
        &self,
    ) -> Option<&CustodyDurableSnapshot> {
        self.pending_predecessor.as_ref()
    }

    pub(in crate::custody_guardian) fn install_pending(
        &mut self,
        preparation: CustodyGuardianPreparation,
        prepared: CustodyDurableRecord,
        predecessor: CustodyDurableSnapshot,
        runtime_record: RuntimeRecord,
        runtime_identity: FileIdentity,
        retained_runtime_file: Option<RetainedRuntimeFile>,
    ) -> Result<()> {
        ensure!(
            prepared.runtime
                == crate::pty_custody::CustodyRuntimeIdentity::from_record(&runtime_record),
            RECORD_DETAIL
        );
        let update =
            self.prepare_runtime_update(&runtime_record, runtime_identity, retained_runtime_file)?;
        self.install_prepared_pending(
            preparation,
            prepared,
            predecessor,
            runtime_record,
            runtime_identity,
            update,
        );
        Ok(())
    }

    pub(in crate::custody_guardian) fn install_prepared_pending(
        &mut self,
        preparation: CustodyGuardianPreparation,
        prepared: CustodyDurableRecord,
        predecessor: CustodyDurableSnapshot,
        runtime_record: RuntimeRecord,
        runtime_identity: FileIdentity,
        runtime_update: super::runtime::PreparedRuntimeUpdate,
    ) {
        self.install_runtime_update(runtime_record, runtime_identity, runtime_update);
        self.pending = Some(preparation);
        self.pending_record = Some(prepared);
        self.pending_predecessor = Some(predecessor);
        self.was_armed = false;
        self.armed_promotion_verified = false;
    }

    pub(in crate::custody_guardian) fn preparation_for(
        &self,
        binding: CustodyGuardianBinding,
    ) -> Result<CustodyGuardianPreparation> {
        self.validate_prepared_binding(binding)?;
        self.pending
            .filter(|pending| pending.binding() == binding)
            .ok_or_else(|| anyhow::anyhow!(RECORD_DETAIL))
    }

    pub(in crate::custody_guardian) fn adopt_prepared_binding(
        &mut self,
        binding: CustodyGuardianBinding,
    ) -> Result<()> {
        self.validate_prepared_binding(binding)?;
        self.binding = binding;
        self.clear_pending();
        self.was_armed = true;
        Ok(())
    }

    pub(in crate::custody_guardian) fn abandon_successor(
        &mut self,
        binding: CustodyGuardianBinding,
    ) -> Result<()> {
        ensure!(
            self.pending
                .is_some_and(|pending| pending.binding() == binding),
            RECORD_DETAIL
        );
        self.clear_pending();
        Ok(())
    }

    pub(in crate::custody_guardian) fn restore_pending(
        &mut self,
        identity: FileIdentity,
    ) -> Result<()> {
        let pending = self.pending.ok_or_else(|| anyhow::anyhow!(RECORD_DETAIL))?;
        self.pending = Some(pending.with_custody_file_identity(identity));
        self.was_armed = false;
        self.armed_promotion_verified = false;
        Ok(())
    }

    fn validate_prepared_binding(&self, binding: CustodyGuardianBinding) -> Result<()> {
        ensure!(
            binding == self.binding
                || self
                    .pending
                    .is_some_and(|pending| pending.binding() == binding),
            RECORD_DETAIL
        );
        ensure!(binding.process().matches(), RECORD_DETAIL);
        Ok(())
    }

    pub(super) fn clear_pending(&mut self) {
        self.pending = None;
        self.pending_record = None;
        self.pending_predecessor = None;
    }
}
