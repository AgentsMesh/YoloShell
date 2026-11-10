use super::{ProcessClosureIdentity, PtyMasterAliasReleaseAuthority};

impl PtyMasterAliasReleaseAuthority {
    pub(in crate::process) fn binds_process_closure(
        &self,
        expected: &ProcessClosureIdentity,
    ) -> bool {
        self.identity.process_closure.same_allocation(expected)
    }

    #[cfg(target_os = "macos")]
    pub fn validates_output_descriptor(&self, descriptor: &std::fs::File) -> anyhow::Result<bool> {
        let observed = super::super::pty_session_proof::inspect_output_pty_identity(descriptor)?;
        Ok(self.identity.object == super::PtyObjectIdentity::Macos(observed))
    }

    #[cfg(any(test, feature = "runtime-component-tests"))]
    pub(in crate::process) fn process_closure_identity_for_test(&self) -> ProcessClosureIdentity {
        self.identity.process_closure.clone()
    }
}
