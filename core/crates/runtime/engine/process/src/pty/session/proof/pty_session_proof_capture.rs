use super::{CapabilityScope, PtySessionProof};
use crate::process::constants::PROCESS_DISCOVERY_CAPABILITY_ENV;
use crate::process::constants::PTY_SESSION_CAPABILITY_ENV;
use anyhow::{Context, Result, ensure};
use std::path::PathBuf;

pub(super) const EMPTY_CAPABILITY_DETAIL: &str = "process capability must not be empty";
const RELATIVE_TTY_PATH_DETAIL: &str = "PTY slave path must be absolute";
#[cfg(any(target_os = "linux", target_os = "macos"))]
const TTY_CANONICALIZE_CONTEXT: &str = "canonicalize PTY slave path";
const TTY_METADATA_CONTEXT: &str = "read PTY slave metadata";
const TTY_TYPE_DETAIL: &str = "PTY slave path is not a character device";
const MASTER_TTY_NAME_CONTEXT: &str = "read PTY slave path from master";
impl PtySessionProof {
    pub(crate) fn capture_bound(
        master: &dyn portable_pty::MasterPty,
        capability: String,
    ) -> Result<Self> {
        let tty_path = master.tty_name().context(MASTER_TTY_NAME_CONTEXT)?;
        let mut proof = Self::capture(tty_path, capability)?;
        #[cfg(target_os = "macos")]
        {
            proof.pty_object_lease = Some(super::macos_object_lease::MacosPtyObjectLease::capture(
                master, &proof,
            )?);
        }
        Ok(proof)
    }

    #[cfg(target_os = "linux")]
    pub(in crate::process) fn linux_device(&self) -> u64 {
        self.tty_device
    }

    #[cfg(target_os = "linux")]
    pub(in crate::process) fn linux_capability(&self) -> &str {
        &self.capability
    }

    #[cfg(target_os = "linux")]
    pub(in crate::process) fn restore_linux(device: u64, capability: String) -> Result<Self> {
        ensure!(device != u64::default(), TTY_TYPE_DETAIL);
        ensure!(!capability.is_empty(), EMPTY_CAPABILITY_DETAIL);
        Ok(Self {
            _tty_path: PathBuf::new(),
            tty_device: device,
            capability,
            runtime: None,
            master_alias_state: super::PtyMasterAliasState::Unbound,
            scope: CapabilityScope::Pty,
            bootstrap: Default::default(),
            bootstrap_exec: Default::default(),
        })
    }

    pub fn capture(tty_path: PathBuf, capability: String) -> Result<Self> {
        use std::os::unix::fs::FileTypeExt;
        ensure!(!capability.is_empty(), EMPTY_CAPABILITY_DETAIL);
        ensure!(tty_path.is_absolute(), RELATIVE_TTY_PATH_DETAIL);
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        let tty_path = std::fs::canonicalize(tty_path).context(TTY_CANONICALIZE_CONTEXT)?;
        ensure!(tty_path.is_absolute(), RELATIVE_TTY_PATH_DETAIL);
        let metadata = std::fs::metadata(&tty_path).context(TTY_METADATA_CONTEXT)?;
        ensure!(metadata.file_type().is_char_device(), TTY_TYPE_DETAIL);
        #[cfg(target_os = "linux")]
        let tty_device = {
            use std::os::unix::fs::MetadataExt;
            metadata.rdev()
        };
        #[cfg(target_os = "macos")]
        let tty_object_identity =
            Some(super::macos_object_identity::MacosPtyObjectIdentity::from_metadata(&metadata));
        Ok(Self {
            #[cfg(target_os = "macos")]
            tty_path,
            #[cfg(target_os = "linux")]
            _tty_path: tty_path,
            #[cfg(target_os = "linux")]
            tty_device,
            #[cfg(target_os = "macos")]
            tty_object_identity,
            #[cfg(target_os = "macos")]
            pty_object_lease: None,
            #[cfg(target_os = "macos")]
            reopen_seal: Default::default(),
            #[cfg(all(target_os = "macos", any(test, feature = "adversarial-test-hooks")))]
            custody_probe: None,
            #[cfg(not(any(target_os = "linux", target_os = "macos")))]
            _tty_path: tty_path,
            capability,
            runtime: None,
            master_alias_state: super::PtyMasterAliasState::Unbound,
            scope: CapabilityScope::Pty,
            bootstrap: Default::default(),
            bootstrap_exec: Default::default(),
            #[cfg(target_os = "macos")]
            descriptor_capability: None,
        })
    }
    pub(crate) fn discovery(capability: String) -> Result<Self> {
        ensure!(!capability.is_empty(), EMPTY_CAPABILITY_DETAIL);
        Ok(Self {
            #[cfg(target_os = "macos")]
            tty_path: PathBuf::new(),
            #[cfg(target_os = "linux")]
            _tty_path: PathBuf::new(),
            #[cfg(target_os = "linux")]
            tty_device: u64::default(),
            #[cfg(target_os = "macos")]
            tty_object_identity: None,
            #[cfg(target_os = "macos")]
            pty_object_lease: None,
            #[cfg(target_os = "macos")]
            reopen_seal: Default::default(),
            #[cfg(all(target_os = "macos", any(test, feature = "adversarial-test-hooks")))]
            custody_probe: None,
            #[cfg(not(any(target_os = "linux", target_os = "macos")))]
            _tty_path: PathBuf::new(),
            capability,
            runtime: None,
            master_alias_state: super::PtyMasterAliasState::NoLocalAlias,
            scope: CapabilityScope::ProcessDiscovery,
            bootstrap: Default::default(),
            bootstrap_exec: Default::default(),
            #[cfg(target_os = "macos")]
            descriptor_capability: None,
        })
    }
    #[cfg(target_os = "macos")]
    pub(in crate::process) fn arm_process_capability(
        &self,
        root: super::super::LiveProcessIdentity,
    ) -> Result<()> {
        if let Some(reader) = self
            .descriptor_capability
            .as_ref()
            .map(super::macos_descriptor_authority::MacosDescriptorAuthority::process_reader)
        {
            reader.arm_liveness(root)?;
        }
        Ok(())
    }

    #[cfg(target_os = "macos")]
    pub(in crate::process) fn confirm_process_capability_root_exit(
        &self,
        root: &super::super::LiveProcessIdentity,
    ) -> Result<()> {
        if let Some(reader) = self
            .descriptor_capability
            .as_ref()
            .map(super::macos_descriptor_authority::MacosDescriptorAuthority::process_reader)
        {
            reader.confirm_root_exit(root)?;
        }
        Ok(())
    }

    pub(in crate::process) fn is_discovery(&self) -> bool {
        matches!(
            self.scope,
            CapabilityScope::ProcessDiscovery | CapabilityScope::CustodyPtyObject
        )
    }

    pub(in crate::process) fn is_process_discovery(&self) -> bool {
        self.scope == CapabilityScope::ProcessDiscovery
    }

    pub(crate) fn requires_pty_object_lease(&self) -> bool {
        cfg!(target_os = "macos") && self.scope == CapabilityScope::Pty
    }

    pub(crate) fn has_required_pty_object_lease(&self) -> bool {
        if !self.requires_pty_object_lease() {
            return true;
        }
        #[cfg(target_os = "macos")]
        return self
            .pty_object_lease
            .as_ref()
            .is_some_and(|lease| lease.matches(self));
        #[cfg(not(target_os = "macos"))]
        false
    }

    #[cfg(target_os = "macos")]
    pub(in crate::process) fn has_transferable_descriptor(&self) -> bool {
        self.descriptor_capability.is_some()
    }

    pub(super) fn environment_name(&self) -> &'static str {
        match self.scope {
            CapabilityScope::Pty => PTY_SESSION_CAPABILITY_ENV,
            CapabilityScope::ProcessDiscovery => PROCESS_DISCOVERY_CAPABILITY_ENV,
            CapabilityScope::CustodyPtyObject => PTY_SESSION_CAPABILITY_ENV,
        }
    }
}
