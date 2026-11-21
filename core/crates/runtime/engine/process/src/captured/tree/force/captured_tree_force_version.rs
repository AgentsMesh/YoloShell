use super::ProcessClosureVersion;
use anyhow::{Context, Result};

const PROCESS_CLOSURE_VERSION_INCREMENT: u64 = 1;
pub(super) const PROCESS_CLOSURE_VERSION_EXHAUSTED_DETAIL: &str =
    "process-tree closure version is exhausted";

impl ProcessClosureVersion {
    pub(in crate::process) fn initial(identity: crate::process::ProcessClosureIdentity) -> Self {
        Self {
            identity,
            generation: u64::default(),
        }
    }

    pub(super) fn next(&self) -> Result<Self> {
        self.generation
            .checked_add(PROCESS_CLOSURE_VERSION_INCREMENT)
            .map(|generation| Self {
                identity: self.identity.clone(),
                generation,
            })
            .context(PROCESS_CLOSURE_VERSION_EXHAUSTED_DETAIL)
    }

    pub fn binds_pty_output(
        &self,
        authority: &crate::process::PtyMasterAliasReleaseAuthority,
    ) -> bool {
        authority.binds_process_closure(&self.identity)
    }

    #[cfg(any(test, feature = "runtime-component-tests"))]
    pub fn for_pty_output_test(
        authority: &crate::process::PtyMasterAliasReleaseAuthority,
    ) -> Self {
        Self::initial(authority.process_closure_identity_for_test())
    }

    #[cfg(any(test, feature = "runtime-component-tests"))]
    pub(super) fn maximum_for_test() -> Self {
        Self {
            identity: crate::process::ProcessClosureIdentity::fresh(),
            generation: u64::MAX,
        }
    }
}

impl Default for ProcessClosureVersion {
    fn default() -> Self {
        Self::initial(crate::process::ProcessClosureIdentity::fresh())
    }
}
