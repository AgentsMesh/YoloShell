impl CodexBindingAuthority {
    pub fn with_verified_resume_identity(mut self, boot_id: &str) -> Result<Self> {
        ensure!(
            self.status == CodexAuthorityStatus::Active,
            AUTHORITY_INVALID
        );
        let resume = super::resume_identity::CodexResumeIdentity::capture(&self.binding, boot_id)?;
        resume.verify()?;
        self.resume_identity = Some(resume);
        self.validate()?;
        Ok(self)
    }

    pub fn recoverable_binding(
        &self,
        runtime: AuthorityRuntimeIdentity,
    ) -> Result<Option<AgentBinding>> {
        self.validate()?;
        if !self.matches_runtime(runtime)
            || !matches!(
                self.status,
                CodexAuthorityStatus::Active | CodexAuthorityStatus::Unavailable
            )
        {
            return Ok(None);
        }
        let Some(resume) = self.resume_identity.as_ref() else {
            return Ok(None);
        };
        resume.verify()?;
        let mut binding = self.binding.clone();
        binding.recovery_availability = AgentRecoveryAvailability::Available;
        Ok(Some(binding))
    }

    pub fn resume_boot_id(&self) -> Option<&str> {
        self.resume_identity
            .as_ref()
            .map(|value| value.boot_id.as_str())
    }

    pub fn verify_resume_evidence(&self) -> Result<()> {
        if let Some(resume) = self.resume_identity.as_ref() {
            resume.verify()?;
        }
        Ok(())
    }

    pub fn inherit_verified_resume_identity(mut self, previous: &Self) -> Result<Self> {
        ensure!(
            self.same_runtime_provider(previous)
                && self.version.route_revision == previous.version.route_revision
                && self.version.invocation_id == previous.version.invocation_id,
            AUTHORITY_INVALID
        );
        self.resume_identity.clone_from(&previous.resume_identity);
        self.validate()?;
        Ok(self)
    }

    pub fn provider_process(&self) -> crate::agent::hooks::ProcessIdentity {
        crate::agent::hooks::ProcessIdentity::new(self.provider_pid, self.provider_start_time)
    }

    pub(super) fn same_runtime_provider(&self, other: &Self) -> bool {
        self.schema_version == other.schema_version
            && self.runtime_epoch == other.runtime_epoch
            && self.worker_pid == other.worker_pid
            && self.worker_start_time == other.worker_start_time
            && self.provider_pid == other.provider_pid
            && self.provider_start_time == other.provider_start_time
            && self.provider_version == other.provider_version
    }

    pub(super) fn same_resume_source(&self, other: &Self) -> bool {
        match (
            self.resume_identity.as_ref(),
            other.resume_identity.as_ref(),
        ) {
            (None, None) => true,
            (Some(left), Some(right)) => left.same_source(right),
            _ => false,
        }
    }

    pub(super) fn resume_progression_is_valid(&self, previous: &Self) -> bool {
        match (
            previous.resume_identity.as_ref(),
            self.resume_identity.as_ref(),
        ) {
            (Some(left), Some(right))
                if self.binding.provider_session_id == previous.binding.provider_session_id =>
            {
                left.allows_successor(right)
            }
            (Some(_), None) => false,
            _ => true,
        }
    }
}
