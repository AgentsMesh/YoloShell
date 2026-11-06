use super::{CapabilityScope, PtyMasterAliasState, PtySessionProof};
use anyhow::{Context, Result, bail, ensure};

const MASTER_ALIAS_RUNTIME_ALREADY_BOUND_DETAIL: &str =
    "PTY master-alias runtime authority is already bound";
const MASTER_ALIAS_RUNTIME_UNAVAILABLE_DETAIL: &str =
    "PTY master-alias runtime authority is unavailable";
const MASTER_ALIAS_STATE_ALREADY_RESOLVED_DETAIL: &str =
    "PTY master-alias state is already resolved";
const MASTER_ALIAS_STATE_UNBOUND_DETAIL: &str = "PTY master-alias state is unresolved";
const MASTER_ALIAS_VERIFIER_UNAVAILABLE_DETAIL: &str =
    "PTY master-alias verifier authority is unavailable";
const MASTER_ALIAS_SCOPE_DETAIL: &str = "PTY master-alias authority requires a PTY capability";
#[cfg(target_os = "macos")]
const MASTER_ALIAS_OBJECT_UNAVAILABLE_DETAIL: &str = "PTY object identity is unavailable";
#[cfg(target_os = "macos")]
const MASTER_ALIAS_OBJECT_SHARED_DETAIL: &str =
    "PTY object lease is still shared while preparing local alias closure";

impl PtySessionProof {
    pub fn bind_runtime(
        &mut self,
        terminal_id: uuid::Uuid,
        runtime_epoch: uuid::Uuid,
    ) -> Result<()> {
        ensure!(
            self.runtime.is_none(),
            MASTER_ALIAS_RUNTIME_ALREADY_BOUND_DETAIL
        );
        self.runtime = Some(super::super::PtyRuntimeIdentity::new(
            terminal_id,
            runtime_epoch,
        ));
        Ok(())
    }

    pub fn bind_master_alias_release(
        &mut self,
        root: super::super::ProcessIdentity,
    ) -> Result<super::super::PtyMasterAliasReleaseAuthority> {
        ensure!(
            self.scope == CapabilityScope::Pty,
            MASTER_ALIAS_SCOPE_DETAIL
        );
        ensure!(
            matches!(&self.master_alias_state, PtyMasterAliasState::Unbound),
            MASTER_ALIAS_STATE_ALREADY_RESOLVED_DETAIL
        );
        let runtime = self
            .runtime
            .context(MASTER_ALIAS_RUNTIME_UNAVAILABLE_DETAIL)?;
        let object = self.master_object_identity()?;
        let (release, verifier) = super::super::PtyMasterAliasReleaseAuthority::new_pair(
            runtime,
            root,
            self.capability.clone(),
            object,
        );
        self.master_alias_state = PtyMasterAliasState::AliasBound(verifier);
        Ok(release)
    }

    pub(in crate::process) fn confirm_no_local_master_aliases(&mut self) -> Result<()> {
        ensure!(
            matches!(&self.master_alias_state, PtyMasterAliasState::Unbound),
            MASTER_ALIAS_STATE_ALREADY_RESOLVED_DETAIL
        );
        self.master_alias_state = PtyMasterAliasState::NoLocalAlias;
        Ok(())
    }

    pub(in crate::process) fn validates_master_aliases_closed(
        &self,
        proof: &super::super::PtyMasterAliasesClosed,
    ) -> bool {
        match &self.master_alias_state {
            PtyMasterAliasState::AliasBound(expected) => proof.validates(expected),
            PtyMasterAliasState::Unbound | PtyMasterAliasState::NoLocalAlias => false,
        }
    }

    pub(in crate::process) fn process_closure_identity(
        &self,
    ) -> Option<super::super::ProcessClosureIdentity> {
        match &self.master_alias_state {
            PtyMasterAliasState::AliasBound(verifier) => Some(verifier.process_closure_identity()),
            PtyMasterAliasState::Unbound | PtyMasterAliasState::NoLocalAlias => None,
        }
    }

    #[cfg(target_os = "macos")]
    pub(in crate::process) fn bind_secondary_custody_recovery(
        &self,
        runtime: &crate::pty_custody::CustodyRuntimeIdentity,
        recovery_identity_sha256: crate::sha256_digest::Sha256Digest,
    ) -> Result<()> {
        match &self.master_alias_state {
            PtyMasterAliasState::AliasBound(verifier) => {
                verifier.bind_secondary_custody_recovery(runtime, recovery_identity_sha256)
            }
            PtyMasterAliasState::Unbound => bail!(MASTER_ALIAS_STATE_UNBOUND_DETAIL),
            PtyMasterAliasState::NoLocalAlias => bail!(MASTER_ALIAS_VERIFIER_UNAVAILABLE_DETAIL),
        }
    }

    #[cfg(target_os = "macos")]
    pub(in crate::process) fn commit_confirmed_bootstrap_global_exclusion(&self) {
        match &self.master_alias_state {
            PtyMasterAliasState::AliasBound(verifier) => {
                verifier.commit_confirmed_bootstrap_global_exclusion();
            }
            PtyMasterAliasState::Unbound | PtyMasterAliasState::NoLocalAlias => {}
        }
    }

    #[cfg(target_os = "macos")]
    pub(in crate::process) fn master_alias_release_requirement(&self) -> Result<bool> {
        match &self.master_alias_state {
            PtyMasterAliasState::AliasBound(_) => Ok(true),
            PtyMasterAliasState::NoLocalAlias => Ok(false),
            PtyMasterAliasState::Unbound => bail!(MASTER_ALIAS_STATE_UNBOUND_DETAIL),
        }
    }

    #[cfg(target_os = "macos")]
    pub(in crate::process) fn release_local_object_lease(&mut self) -> Result<()> {
        ensure!(
            matches!(&self.master_alias_state, PtyMasterAliasState::AliasBound(_)),
            MASTER_ALIAS_VERIFIER_UNAVAILABLE_DETAIL
        );
        let seal_prepared = self.reopen_seal_prepared_for_master_alias_close();
        if let Some(lease) = self.pty_object_lease.as_ref() {
            ensure!(lease.is_exclusive(), MASTER_ALIAS_OBJECT_SHARED_DETAIL);
        } else {
            ensure!(seal_prepared, MASTER_ALIAS_OBJECT_UNAVAILABLE_DETAIL);
        }
        if !seal_prepared {
            self.seal_reopen_before_master_alias_close()?;
        }
        drop(self.pty_object_lease.take());
        Ok(())
    }

    #[cfg(target_os = "macos")]
    pub(super) fn release_no_alias_object_lease(&mut self) -> Result<()> {
        ensure!(
            matches!(&self.master_alias_state, PtyMasterAliasState::NoLocalAlias),
            MASTER_ALIAS_STATE_UNBOUND_DETAIL
        );
        if let Some(lease) = self.pty_object_lease.as_ref() {
            ensure!(lease.is_exclusive(), MASTER_ALIAS_OBJECT_SHARED_DETAIL);
        }
        if !self.reopen_seal_prepared_for_master_alias_close() {
            self.seal_reopen_before_master_alias_close()?;
        }
        drop(self.pty_object_lease.take());
        Ok(())
    }

    fn master_object_identity(
        &self,
    ) -> Result<super::super::pty_master_release::PtyObjectIdentity> {
        #[cfg(target_os = "linux")]
        return Ok(
            super::super::pty_master_release::PtyObjectIdentity::LinuxDevice(self.tty_device),
        );
        #[cfg(target_os = "macos")]
        return Ok(super::super::pty_master_release::PtyObjectIdentity::Macos(
            self.tty_object_identity
                .context(MASTER_ALIAS_OBJECT_UNAVAILABLE_DETAIL)?,
        ));
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        Ok(
            super::super::pty_master_release::PtyObjectIdentity::PortablePath(
                self._tty_path.clone(),
            ),
        )
    }
}
