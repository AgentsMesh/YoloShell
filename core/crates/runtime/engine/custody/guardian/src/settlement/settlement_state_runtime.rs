use super::GuardianState;
use crate::model::RuntimeRecord;
use crate::paths::identity::FileIdentity;
use crate::pty_custody::CustodyRuntimeIdentity;
use anyhow::{Context, Result, ensure};

use super::super::settlement::RECORD_DETAIL;
use super::super::settlement_runtime_file::RetainedRuntimeFile;

pub(super) enum RetainedRuntime {
    Exact {
        record: RuntimeRecord,
        identity: FileIdentity,
        _file: RetainedRuntimeFile,
    },
    #[cfg(test)]
    PreparedOnly,
}

pub(in crate::custody_guardian) struct PreparedRuntimeUpdate {
    retained_file: Option<RetainedRuntimeFile>,
}

impl GuardianState {
    pub(in crate::custody_guardian) fn runtime(&self) -> &CustodyRuntimeIdentity {
        &self.runtime
    }

    pub(in crate::custody_guardian) fn retained_runtime(
        &self,
    ) -> Result<(&RuntimeRecord, FileIdentity)> {
        match &self.retained_runtime {
            RetainedRuntime::Exact {
                record, identity, ..
            } => Ok((record, *identity)),
            #[cfg(test)]
            RetainedRuntime::PreparedOnly => anyhow::bail!(RECORD_DETAIL),
        }
    }

    pub(in crate::custody_guardian) fn runtime_replacement_required(
        &self,
        record: &RuntimeRecord,
        identity: FileIdentity,
    ) -> Result<bool> {
        let (retained, retained_identity) = self.retained_runtime()?;
        record
            .validate_update_from(retained)
            .context(RECORD_DETAIL)?;
        ensure!(
            CustodyRuntimeIdentity::from_record(record) == self.runtime,
            RECORD_DETAIL
        );
        if record == retained {
            ensure!(identity == retained_identity, RECORD_DETAIL);
            Ok(false)
        } else {
            ensure!(!identity.same_object(retained_identity), RECORD_DETAIL);
            Ok(true)
        }
    }

    pub(in crate::custody_guardian) fn adopt_activated_runtime(
        &mut self,
        record: RuntimeRecord,
        identity: FileIdentity,
        retained_file: Option<RetainedRuntimeFile>,
    ) -> Result<()> {
        let update = self.prepare_runtime_update(&record, identity, retained_file)?;
        self.install_runtime_update(record, identity, update);
        Ok(())
    }

    pub(in crate::custody_guardian) fn prepare_runtime_update(
        &self,
        record: &RuntimeRecord,
        identity: FileIdentity,
        retained_file: Option<RetainedRuntimeFile>,
    ) -> Result<PreparedRuntimeUpdate> {
        let replacement_required = self.runtime_replacement_required(record, identity)?;
        ensure!(
            replacement_required == retained_file.is_some(),
            RECORD_DETAIL
        );
        Ok(PreparedRuntimeUpdate { retained_file })
    }

    pub(super) fn install_runtime_update(
        &mut self,
        record: RuntimeRecord,
        identity: FileIdentity,
        update: PreparedRuntimeUpdate,
    ) {
        if let Some(file) = update.retained_file {
            self.retained_runtime = RetainedRuntime::Exact {
                record,
                identity,
                _file: file,
            };
        }
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/settlement/settlement_state_runtime_tests.rs"]
mod tests;
