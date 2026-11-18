use super::{AgentBinding, ClaudeNativeGeneration, ClaudeNativeGenerationStore, INVALID};
use crate::namespace_lease::NamespaceLease;
use crate::paths::identity::FileIdentity;
use anyhow::{Context, Result, ensure};

impl ClaudeNativeGenerationStore {
    pub fn publish_binding_in_namespace(
        &self,
        namespace: &NamespaceLease,
        expected: &ClaudeNativeGeneration,
        mut fence: impl FnMut() -> Result<()>,
    ) -> Result<()> {
        let mutation = super::super::super::mutation::GenerationMutation::acquire(
            &self.generation,
            namespace,
        )?;
        let source = self
            .load_with_identity_in_namespace(namespace)?
            .context(INVALID)?;
        ensure!(
            &source.0 == expected && (expected.revoked || expected.verify_resume_evidence()?),
            INVALID
        );
        let existing: Option<(AgentBinding, FileIdentity)> =
            if namespace.file_is_missing(&self.binding_path)? {
                None
            } else {
                Some(namespace.read_json_with_identity(&self.binding_path)?)
            };
        let target = expected.recovery_binding(true);
        let mut verify = || -> Result<()> {
            fence()?;
            mutation.require_current()?;
            ensure!(
                self.load_with_identity_in_namespace(namespace)?.as_ref() == Some(&source)
                    && (expected.revoked || expected.verify_resume_evidence()?),
                INVALID
            );
            Ok(())
        };
        if existing
            .as_ref()
            .is_some_and(|(binding, _)| binding == &target)
        {
            mutation.require_current()?;
            verify()?;
            let current: (AgentBinding, FileIdentity) =
                namespace.read_json_with_identity(&self.binding_path)?;
            ensure!(Some(&current) == existing.as_ref(), INVALID);
            mutation.require_current()?;
            return Ok(());
        }
        // Missing projection is repaired only under the existing source and
        // session mutation authority; a raced-in file cannot be overwritten.
        mutation.write_path(
            namespace,
            &self.binding_path,
            existing.as_ref().map(|(_, identity)| *identity),
            &target,
            verify,
        )
    }
}
