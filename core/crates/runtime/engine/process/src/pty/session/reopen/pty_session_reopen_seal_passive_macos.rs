use super::{
    MacosPtyReopenSeal, SEAL_IDENTITY_DETAIL, SEALED_MODE, SealAuthorityIdentity, SealPhase,
};
use crate::process::pty_session_proof::{
    macos_object_identity::MacosPtyObjectIdentity, macos_passive_master::MacosPassivePtyMaster,
    macos_reopen_seal_io as io, macos_reopen_seal_master::MacosPtyReopenMasterAuthority,
};
use anyhow::{Result, ensure};
use std::sync::Arc;

#[derive(Clone)]
pub(in crate::process) struct MacosCommittedPtyReopenSeal {
    authority: Arc<SealAuthorityIdentity>,
    object: MacosPtyObjectIdentity,
}

#[derive(Clone)]
pub(in crate::process) struct MacosFinalizedPtyReopenSeal {
    committed: MacosCommittedPtyReopenSeal,
}

impl MacosPtyReopenSeal {
    pub(in crate::process) fn prepare_passive(passive: &MacosPassivePtyMaster) -> Result<Self> {
        let master = passive.duplicate_self()?;
        let path = master.slave_path().to_path_buf();
        let object = master.object_identity();
        if io::exact_path_is_sealed(&path, object, SEALED_MODE)? {
            return Ok(Self {
                master: Some(MacosPtyReopenMasterAuthority::Local(master)),
                temporary_slave: None,
                path,
                object,
                original_mode: SEALED_MODE,
                authority: Arc::new(SealAuthorityIdentity),
                phase: SealPhase::Committed,
            });
        }
        let slave = io::open_slave(&path)?;
        let status = io::descriptor_status(&slave)?;
        ensure!(status.identity == object, SEAL_IDENTITY_DETAIL);
        Ok(Self {
            master: Some(MacosPtyReopenMasterAuthority::Local(master)),
            temporary_slave: Some(slave),
            path,
            object,
            original_mode: status.mode,
            authority: Arc::new(SealAuthorityIdentity),
            phase: SealPhase::Prepared,
        })
    }

    pub(in crate::process) fn capture_passive_holders(
        &self,
        passive: &MacosPassivePtyMaster,
        exclusions: &super::super::macos_passive_master::MacosPtyHolderExclusions,
    ) -> Result<Vec<crate::process::ProcessIdentity>> {
        ensure!(
            matches!(
                self.phase,
                SealPhase::Committed | SealPhase::PassiveFinalized
            ),
            SEAL_IDENTITY_DETAIL
        );
        self.revalidate_passive(passive)?;
        passive.exact_holder_identities_at(&self.path, exclusions)
    }

    pub(in crate::process) fn commit_passive(
        &mut self,
        passive: &MacosPassivePtyMaster,
    ) -> Result<()> {
        self.revalidate_passive(passive)?;
        if self.phase == SealPhase::Acquired {
            self.phase = SealPhase::Committed;
        }
        ensure!(
            matches!(
                self.phase,
                SealPhase::Committed | SealPhase::PassiveFinalized
            ),
            SEAL_IDENTITY_DETAIL
        );
        Ok(())
    }

    pub(in crate::process) fn committed_passive(
        &self,
        passive: &MacosPassivePtyMaster,
    ) -> Result<MacosCommittedPtyReopenSeal> {
        ensure!(
            matches!(
                self.phase,
                SealPhase::Committed | SealPhase::PassiveFinalized
            ),
            SEAL_IDENTITY_DETAIL
        );
        self.revalidate_passive(passive)?;
        Ok(MacosCommittedPtyReopenSeal {
            authority: Arc::clone(&self.authority),
            object: self.object,
        })
    }

    pub(in crate::process) fn finalize_passive(
        &mut self,
        passive: &MacosPassivePtyMaster,
    ) -> Result<MacosFinalizedPtyReopenSeal> {
        self.revalidate_passive(passive)?;
        if self.phase == SealPhase::Committed {
            drop(self.temporary_slave.take());
            self.phase = SealPhase::PassiveFinalized;
        }
        ensure!(
            self.phase == SealPhase::PassiveFinalized,
            SEAL_IDENTITY_DETAIL
        );
        Ok(MacosFinalizedPtyReopenSeal {
            committed: self.committed_passive(passive)?,
        })
    }

    fn revalidate_passive(&self, passive: &MacosPassivePtyMaster) -> Result<()> {
        passive.revalidate_self()?;
        let master = self
            .master
            .as_ref()
            .and_then(MacosPtyReopenMasterAuthority::passive)
            .ok_or_else(|| anyhow::anyhow!(SEAL_IDENTITY_DETAIL))?;
        master.revalidate_self()?;
        ensure!(
            master.slave_path() == passive.slave_path()
                && master.object_identity() == passive.object_identity()
                && self.path == passive.slave_path()
                && self.object == passive.object_identity(),
            SEAL_IDENTITY_DETAIL
        );
        io::validate_path(&self.path, self.object, SEALED_MODE)
    }
}

impl MacosCommittedPtyReopenSeal {
    pub(in crate::process) fn same_authority(&self, other: &Self) -> bool {
        self.object == other.object && Arc::ptr_eq(&self.authority, &other.authority)
    }

    pub fn validates(&self, other: &Self, object: MacosPtyObjectIdentity) -> bool {
        self.object == object
            && other.object == object
            && Arc::ptr_eq(&self.authority, &other.authority)
    }
}

impl MacosFinalizedPtyReopenSeal {
    pub fn validates(
        &self,
        committed: &MacosCommittedPtyReopenSeal,
        object: MacosPtyObjectIdentity,
    ) -> bool {
        self.committed.validates(committed, object)
    }
}
