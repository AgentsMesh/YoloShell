use super::ClaudeGenerationAuthority;
use crate::agent::{AgentBinding, ObservedProcess};
use anyhow::{Result, ensure};
use std::path::{Path, PathBuf};

pub trait HookPublicationFence: Send + Sync {
    fn verify(&self, authority: &ClaudeGenerationAuthority) -> Result<()>;
}

pub struct HookPublicationAdmission {
    authority: ClaudeGenerationAuthority,
    observed: ObservedProcess,
    transcript_path: PathBuf,
    transcript_identity: crate::paths::identity::ObjectIdentity,
    managed_shim_exec: Option<super::managed_shim_exec_publication::ManagedShimExecPublication>,
    fence: Box<dyn HookPublicationFence>,
}

impl HookPublicationAdmission {
    pub fn new(
        authority: &ClaudeGenerationAuthority,
        observed: ObservedProcess,
        fence: impl HookPublicationFence + 'static,
    ) -> Result<Self> {
        let (transcript_path, transcript_identity) =
            super::observed_unmanaged_publication_validation::authority_transcript(authority)?;
        Self::build(
            authority,
            observed,
            transcript_path,
            transcript_identity,
            None,
            fence,
        )
    }

    pub fn for_session_end(
        authority: &ClaudeGenerationAuthority,
        observed: ObservedProcess,
        transcript_path: PathBuf,
        transcript_identity: crate::paths::identity::ObjectIdentity,
        fence: impl HookPublicationFence + 'static,
    ) -> Result<Self> {
        Self::build(
            authority,
            observed,
            transcript_path,
            transcript_identity,
            None,
            fence,
        )
    }

    pub fn for_predecessor(
        authority: &ClaudeGenerationAuthority,
        observed: ObservedProcess,
        predecessor: Option<&AgentBinding>,
        managed_shim: &Path,
        fence: impl HookPublicationFence + 'static,
    ) -> Result<Self> {
        let managed_shim_exec =
            super::managed_shim_exec_publication::ManagedShimExecPublication::capture(
                authority,
                &observed,
                predecessor,
                managed_shim,
            );
        let (transcript_path, transcript_identity) =
            super::observed_unmanaged_publication_validation::authority_transcript(authority)?;
        Self::build(
            authority,
            observed,
            transcript_path,
            transcript_identity,
            managed_shim_exec,
            fence,
        )
    }

    fn build(
        authority: &ClaudeGenerationAuthority,
        observed: ObservedProcess,
        transcript_path: PathBuf,
        transcript_identity: crate::paths::identity::ObjectIdentity,
        managed_shim_exec: Option<super::managed_shim_exec_publication::ManagedShimExecPublication>,
        fence: impl HookPublicationFence + 'static,
    ) -> Result<Self> {
        let admission = Self {
            authority: authority.clone(),
            observed,
            transcript_path,
            transcript_identity,
            managed_shim_exec,
            fence: Box::new(fence),
        };
        admission.verify_captured()?;
        Ok(admission)
    }

    pub(super) fn accepts(&self, existing: Option<&AgentBinding>) -> Result<bool> {
        self.verify_captured()?;
        Ok(existing.is_some_and(|binding| {
            super::observed_unmanaged_publication_validation::matches_binding(
                &self.authority,
                &self.observed,
                binding,
                &self.transcript_path,
            )
        }) || self
            .managed_shim_exec
            .as_ref()
            .is_some_and(|admission| admission.accepts(existing)))
    }

    pub(super) fn verify_captured(&self) -> Result<()> {
        self.verify_against(&self.authority)
    }

    pub(super) fn verify_against(&self, authority: &ClaudeGenerationAuthority) -> Result<()> {
        ensure!(
            same_authority_subject(&self.authority, authority),
            super::constants::DURABLE_BINDING_INVALID
        );
        ensure!(
            super::observed_unmanaged_publication_validation::matches_publishable_authority(
                authority,
                &self.transcript_path,
            ) && super::observed_unmanaged_publication_validation::matches_process(
                authority,
                &self.observed,
            ),
            super::constants::DURABLE_BINDING_INVALID
        );
        if let Some(admission) = self.managed_shim_exec.as_ref() {
            admission.verify(&self.authority, &self.observed)?;
        }
        self.fence.verify(authority)?;
        ensure!(
            crate::agent::claude::durable_transcript_identity(
                &authority.data_home,
                &self.transcript_path,
            )? == Some(self.transcript_identity),
            super::constants::DURABLE_BINDING_INVALID
        );
        Ok(())
    }
}

fn same_authority_subject(
    expected: &ClaudeGenerationAuthority,
    actual: &ClaudeGenerationAuthority,
) -> bool {
    let mut expected = expected.clone();
    let mut actual = actual.clone();
    expected.binding_revision = None;
    actual.binding_revision = None;
    expected == actual
}

#[cfg(test)]
pub(super) fn admit_observed_publication_for_test(
    authority: &ClaudeGenerationAuthority,
    process: ObservedProcess,
    fence: impl HookPublicationFence + 'static,
) -> Result<HookPublicationAdmission> {
    HookPublicationAdmission::new(authority, process, fence)
}
