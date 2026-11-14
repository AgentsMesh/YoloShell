use super::PtySessionProof;
use super::macos_object_identity::MacosPtyObjectIdentity;
use super::macos_reopen_seal_io as io;
use super::macos_reopen_seal_master::MacosPtyReopenMasterAuthority;
#[cfg(any(test, feature = "runtime-component-tests"))]
use super::macos_reopen_seal_panic::{self as panic_hook, TestSealOperation};
use anyhow::{Result, ensure};
use std::os::fd::OwnedFd;
use std::path::PathBuf;
use std::sync::Arc;

const SEALED_MODE: libc::mode_t = 0;
const SEAL_IDENTITY_DETAIL: &str = "PTY reopen seal belongs to another exact slave object";
const SEAL_MASTER_RELEASED_DETAIL: &str =
    "PTY reopen seal lost its exact master before alias closure";
const TEMPORARY_SLAVE_UNAVAILABLE_DETAIL: &str = "PTY reopen seal temporary slave is unavailable";
const SEAL_NOT_COMMITTED_DETAIL: &str =
    "PTY reopen seal cannot close master aliases before destructive commit";
const SEAL_ALIAS_CLOSE_DETAIL: &str = "PTY reopen seal did not prepare exact master-alias closure";
const SEAL_DROP_RESTORE_LOG: &str = "failed to restore an uncommitted PTY reopen seal";

#[derive(Clone, Copy, Eq, PartialEq)]
enum SealPhase {
    Prepared,
    Acquired,
    FenceCrossed,
    Committed,
    PassiveFinalized,
    AliasesPrepared,
    Restored,
}

pub(super) struct MacosPtyReopenSeal {
    master: Option<MacosPtyReopenMasterAuthority>,
    temporary_slave: Option<OwnedFd>,
    path: PathBuf,
    object: MacosPtyObjectIdentity,
    original_mode: libc::mode_t,
    authority: Arc<SealAuthorityIdentity>,
    phase: SealPhase,
}

#[derive(Clone)]
pub(super) struct MacosPtyReopenSealIdentity(Arc<SealAuthorityIdentity>);

struct SealAuthorityIdentity;

impl MacosPtyReopenSeal {
    pub(super) fn prepare(proof: &PtySessionProof) -> Result<Self> {
        let master = MacosPtyReopenMasterAuthority::capture(proof)?;
        let path = master.slave_path().to_path_buf();
        let object = master.object_identity();
        let slave = io::open_slave(&path)?;
        let status = io::descriptor_status(&slave)?;
        ensure!(status.identity == object, SEAL_IDENTITY_DETAIL);
        Ok(Self {
            master: Some(master),
            temporary_slave: Some(slave),
            path,
            object,
            original_mode: status.mode,
            authority: Arc::new(SealAuthorityIdentity),
            phase: SealPhase::Prepared,
        })
    }

    pub(super) fn activate(&mut self) -> Result<()> {
        #[cfg(any(test, feature = "runtime-component-tests"))]
        panic_hook::panic_if_requested(TestSealOperation::Activate);
        if self.phase == SealPhase::Prepared {
            io::set_descriptor_mode(self.temporary_slave(), SEALED_MODE)?;
            self.phase = SealPhase::Acquired;
        }
        ensure!(
            matches!(self.phase, SealPhase::Acquired | SealPhase::Committed),
            SEAL_IDENTITY_DETAIL
        );
        if let Some(slave) = self.temporary_slave.as_ref() {
            #[cfg(any(test, feature = "runtime-component-tests"))]
            io::validate_seal_commit()?;
            let sealed = io::descriptor_status(slave)?;
            ensure!(sealed.identity == self.object, SEAL_IDENTITY_DETAIL);
            ensure!(sealed.mode == SEALED_MODE, SEAL_IDENTITY_DETAIL);
        }
        Ok(())
    }

    pub(super) fn identity(&self) -> MacosPtyReopenSealIdentity {
        MacosPtyReopenSealIdentity(Arc::clone(&self.authority))
    }

    pub(super) fn has_identity(&self, identity: &MacosPtyReopenSealIdentity) -> bool {
        Arc::ptr_eq(&self.authority, &identity.0)
    }

    pub(super) fn revalidate(&self, proof: &PtySessionProof) -> Result<()> {
        #[cfg(any(test, feature = "runtime-component-tests"))]
        panic_hook::panic_if_requested(TestSealOperation::Revalidate);
        self.validate_authority(proof)?;
        io::validate_path(&self.path, self.object, SEALED_MODE)
    }

    pub(super) fn validate_authority(&self, proof: &PtySessionProof) -> Result<()> {
        let master = self
            .master
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!(SEAL_MASTER_RELEASED_DETAIL))?;
        master.revalidate(proof)?;
        ensure!(
            master.slave_path() == self.path && master.object_identity() == self.object,
            SEAL_IDENTITY_DETAIL
        );
        Ok(())
    }

    pub(super) fn revalidate_committed(&self, proof: &PtySessionProof) -> Result<()> {
        ensure!(
            self.phase == SealPhase::Committed,
            SEAL_NOT_COMMITTED_DETAIL
        );
        self.revalidate(proof)
    }

    pub(super) fn requires_fresh_eof(&self) -> bool {
        matches!(
            self.phase,
            SealPhase::Prepared | SealPhase::Acquired | SealPhase::FenceCrossed
        )
    }

    pub(super) fn prepare_master_alias_close(&mut self, proof: &PtySessionProof) -> Result<()> {
        #[cfg(any(test, feature = "runtime-component-tests"))]
        panic_hook::panic_if_requested(TestSealOperation::PrepareMasterAliasClose);
        ensure!(
            self.phase == SealPhase::Committed,
            SEAL_NOT_COMMITTED_DETAIL
        );
        self.revalidate(proof)?;
        drop(self.master.take());
        self.phase = SealPhase::AliasesPrepared;
        Ok(())
    }

    pub(super) fn validate_master_alias_close(&self) -> Result<()> {
        ensure!(
            self.phase == SealPhase::AliasesPrepared,
            SEAL_ALIAS_CLOSE_DETAIL
        );
        Ok(())
    }

    pub(super) fn master_alias_close_is_prepared(&self) -> bool {
        self.phase == SealPhase::AliasesPrepared
    }

    pub(super) fn restore_before_commit(&mut self) -> Result<()> {
        #[cfg(any(test, feature = "runtime-component-tests"))]
        panic_hook::panic_if_requested(TestSealOperation::Restore);
        if matches!(self.phase, SealPhase::Prepared | SealPhase::Restored) {
            self.phase = SealPhase::Restored;
            return Ok(());
        }
        ensure!(self.phase == SealPhase::Acquired, SEAL_NOT_COMMITTED_DETAIL);
        io::set_descriptor_mode(self.temporary_slave(), self.original_mode)?;
        self.phase = SealPhase::Restored;
        Ok(())
    }

    fn temporary_slave(&self) -> &OwnedFd {
        self.temporary_slave
            .as_ref()
            .expect(TEMPORARY_SLAVE_UNAVAILABLE_DETAIL)
    }

    #[cfg(any(test, feature = "runtime-component-tests"))]
    pub(super) fn replace_validation_path_for_test(&mut self, path: PathBuf) {
        self.path = path;
    }
}

impl MacosPtyReopenSealIdentity {
    pub(super) fn same_authority(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

#[path = "pty_session_reopen_seal_commit_macos.rs"]
mod commit;
#[path = "pty_session_reopen_seal_drop_macos.rs"]
mod drop_containment;
#[path = "pty_session_reopen_seal_passive_macos.rs"]
mod passive;
pub(in crate::process) use passive::MacosCommittedPtyReopenSeal;
pub(in crate::process) use passive::MacosFinalizedPtyReopenSeal;
#[path = "pty_session_reopen_seal_repair_macos.rs"]
mod repair;

#[cfg(all(test, feature = "yolo-integration-tests"))]
include!("../../../../integration_tests/pty/session/reopen/pty_session_reopen_seal_macos_test_wiring.rs");
