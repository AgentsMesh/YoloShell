use super::*;

impl CodexBindingAuthority {
    pub fn new(
        runtime: AuthorityRuntimeIdentity,
        provider: crate::agent::hooks::ProcessIdentity,
        provider_version: ProviderVersion,
        version: CodexAuthorityVersion,
        status: CodexAuthorityStatus,
        binding: AgentBinding,
    ) -> Result<Self> {
        Self::build(
            runtime,
            provider,
            provider_version,
            version,
            status,
            binding,
            None,
        )
    }

    /// Captures the resume evidence before validating an exited authority.
    pub fn new_with_verified_resume_identity(
        runtime: AuthorityRuntimeIdentity,
        provider: crate::agent::hooks::ProcessIdentity,
        provider_version: ProviderVersion,
        version: CodexAuthorityVersion,
        status: CodexAuthorityStatus,
        binding: AgentBinding,
        boot_id: &str,
    ) -> Result<Self> {
        ensure!(status == CodexAuthorityStatus::Active, AUTHORITY_INVALID);
        Self::build(
            runtime,
            provider,
            provider_version,
            version,
            status,
            binding,
            Some(boot_id),
        )
    }

    fn build(
        runtime: AuthorityRuntimeIdentity,
        provider: crate::agent::hooks::ProcessIdentity,
        provider_version: ProviderVersion,
        version: CodexAuthorityVersion,
        status: CodexAuthorityStatus,
        binding: AgentBinding,
        boot_id: Option<&str>,
    ) -> Result<Self> {
        let resume_identity = boot_id
            .map(|boot| {
                let resume =
                    super::super::resume_identity::CodexResumeIdentity::capture(&binding, boot)?;
                resume.verify()?;
                Ok::<_, anyhow::Error>(resume)
            })
            .transpose()?;
        let authority = Self {
            schema_version: super::super::AUTHORITY_SCHEMA_VERSION,
            runtime_epoch: runtime.runtime_epoch,
            worker_pid: runtime.worker.pid,
            worker_start_time: runtime.worker.start_time,
            provider_pid: provider.pid,
            provider_start_time: provider.start_time,
            provider_version,
            version,
            status,
            binding,
            resume_identity,
        };
        authority.validate()?;
        Ok(authority)
    }
}
