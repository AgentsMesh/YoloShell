use super::{CapabilityScope, PtyMasterAliasState, PtySessionProof};

const BOUND_PROOF_CLONE_DETAIL: &str = "bound PTY master-alias verifier authority cannot be cloned";
#[cfg(target_os = "macos")]
const SEALED_PROOF_CLONE_DETAIL: &str = "sealed PTY reopen authority cannot be cloned";

impl Clone for PtySessionProof {
    fn clone(&self) -> Self {
        assert!(
            !matches!(&self.master_alias_state, PtyMasterAliasState::AliasBound(_)),
            "{BOUND_PROOF_CLONE_DETAIL}"
        );
        #[cfg(target_os = "macos")]
        assert!(self.reopen_seal.is_idle(), "{SEALED_PROOF_CLONE_DETAIL}");
        Self {
            #[cfg(target_os = "macos")]
            tty_path: self.tty_path.clone(),
            #[cfg(target_os = "linux")]
            _tty_path: self._tty_path.clone(),
            #[cfg(target_os = "linux")]
            tty_device: self.tty_device,
            #[cfg(target_os = "macos")]
            tty_object_identity: self.tty_object_identity,
            #[cfg(target_os = "macos")]
            pty_object_lease: self.pty_object_lease.clone(),
            #[cfg(target_os = "macos")]
            reopen_seal: Default::default(),
            #[cfg(all(target_os = "macos", any(test, feature = "adversarial-test-hooks")))]
            custody_probe: self.custody_probe.clone(),
            #[cfg(not(any(target_os = "linux", target_os = "macos")))]
            _tty_path: self._tty_path.clone(),
            capability: self.capability.clone(),
            runtime: self.runtime,
            master_alias_state: match &self.master_alias_state {
                PtyMasterAliasState::Unbound => PtyMasterAliasState::Unbound,
                PtyMasterAliasState::NoLocalAlias
                    if matches!(
                        self.scope,
                        CapabilityScope::ProcessDiscovery | CapabilityScope::CustodyPtyObject
                    ) =>
                {
                    PtyMasterAliasState::NoLocalAlias
                }
                PtyMasterAliasState::NoLocalAlias | PtyMasterAliasState::AliasBound(_) => {
                    panic!("{BOUND_PROOF_CLONE_DETAIL}")
                }
            },
            scope: self.scope,
            bootstrap: self.bootstrap.without_retained_coverage(),
            bootstrap_exec: self.bootstrap_exec.clone(),
            #[cfg(target_os = "macos")]
            descriptor_capability: self.descriptor_capability.clone(),
        }
    }
}
