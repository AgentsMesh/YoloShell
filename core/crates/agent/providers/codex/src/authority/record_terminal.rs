impl CodexBindingAuthority {
    #[cfg(any(test, feature = "adversarial-test-hooks"))]
    pub(crate) fn with_adversarial_active_state(&self, state: AgentState) -> Result<Self> {
        let mut authority = self.clone();
        let has_process = state != AgentState::Exited;
        authority.status = CodexAuthorityStatus::Active;
        authority.binding.state = state;
        authority.binding.recovery_availability = AgentRecoveryAvailability::Available;
        authority.binding.process_pid = has_process.then_some(authority.provider_pid);
        authority.binding.process_start_time = has_process.then_some(authority.provider_start_time);
        authority.validate()?;
        Ok(authority)
    }

    pub(crate) fn unavailable(&self) -> Result<Self> {
        self.terminal(CodexAuthorityStatus::Unavailable, true)
    }

    pub(crate) fn revoked(&self) -> Result<Self> {
        self.terminal(CodexAuthorityStatus::Revoked, false)
    }

    fn terminal(&self, status: CodexAuthorityStatus, advance_time: bool) -> Result<Self> {
        let mut authority = self.clone();
        authority.status = status;
        authority.binding.state = AgentState::Unknown;
        authority.binding.recovery_availability = AgentRecoveryAvailability::Unavailable;
        authority.binding.process_pid = Some(authority.provider_pid);
        authority.binding.process_start_time = Some(authority.provider_start_time);
        if advance_time {
            authority.binding.updated_at_ms = crate::model::now_ms()
                .max(authority.binding.detected_at_ms)
                .max(authority.binding.updated_at_ms);
        }
        authority.validate()?;
        Ok(authority)
    }
}
