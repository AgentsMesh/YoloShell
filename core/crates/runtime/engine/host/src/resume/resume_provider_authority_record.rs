use super::*;

pub(crate) const RESUME_PROVIDER_AUTHORITY_INVALID: &str = "provider resume authority is invalid";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProviderResumeAuthority {
    pub(crate) schema_version: u16,
    #[serde(with = "crate::serde_uuid")]
    pub(crate) runtime_epoch: Uuid,
    pub(crate) runtime_child_pid: u32,
    pub(crate) runtime_child_start_time: u64,
    pub(crate) binding: AgentBinding,
    pub(crate) observed_at_ms: i64,
    pub(crate) authority_digest: String,
}

#[derive(Serialize)]
pub(crate) struct ProviderResumeAuthorityDigestBody {
    pub(crate) schema_version: u16,
    pub(crate) transaction_digest: String,
    #[serde(with = "crate::serde_uuid")]
    pub(crate) runtime_epoch: Uuid,
    pub(crate) runtime_child_pid: u32,
    pub(crate) runtime_child_start_time: u64,
    pub(crate) binding: AgentBinding,
    pub(crate) observed_at_ms: i64,
}

impl ProviderResumeAuthority {
    pub(crate) fn new(
        transaction: &ResumeTransaction,
        runtime: &RuntimeRecord,
        binding: AgentBinding,
        observed_at_ms: i64,
    ) -> Result<Self> {
        let mut authority = Self {
            schema_version: crate::constants::PROVIDER_RESUME_AUTHORITY_SCHEMA_VERSION,
            runtime_epoch: runtime.runtime_epoch,
            runtime_child_pid: runtime.child_pid,
            runtime_child_start_time: runtime.child_start_time,
            binding,
            observed_at_ms,
            authority_digest: String::new(),
        };
        authority.authority_digest = provider_resume_authority_digest(transaction, &authority)?;
        authority.validate_for(transaction)?;
        authority.validate_runtime(runtime)?;
        Ok(authority)
    }

    pub(crate) fn validate_for(&self, transaction: &ResumeTransaction) -> Result<()> {
        let ResumeTarget::Provider {
            binding: planned, ..
        } = &transaction.target
        else {
            anyhow::bail!(RESUME_PROVIDER_AUTHORITY_INVALID);
        };
        let current_identity = self
            .binding
            .process_pid
            .zip(self.binding.process_start_time);
        let planned_identity = planned.process_pid.zip(planned.process_start_time);
        anyhow::ensure!(
            self.schema_version == crate::constants::PROVIDER_RESUME_AUTHORITY_SCHEMA_VERSION
                && self.runtime_epoch == transaction.planned_runtime_epoch
                && self.runtime_child_pid != 0
                && self.runtime_child_start_time != 0
                && self.binding.detected_at_ms >= 0
                && self.binding.updated_at_ms >= self.binding.detected_at_ms
                && self.observed_at_ms >= self.binding.updated_at_ms
                && provider_binding_stable_fields_match(planned, &self.binding)
                && current_identity.is_some()
                && current_identity != planned_identity
                && self.binding.recovery_availability
                    == crate::agent::AgentRecoveryAvailability::Available
                && self.binding.state.is_active(),
            RESUME_PROVIDER_AUTHORITY_INVALID
        );
        anyhow::ensure!(
            valid_lower_hex(&self.authority_digest, SHA256_HEX_LENGTH)
                && provider_resume_authority_digest(transaction, self)? == self.authority_digest,
            RESUME_PROVIDER_AUTHORITY_INVALID
        );
        Ok(())
    }

    pub(crate) fn validate_runtime(&self, runtime: &RuntimeRecord) -> Result<()> {
        anyhow::ensure!(
            runtime.runtime_epoch == self.runtime_epoch
                && runtime.child_pid == self.runtime_child_pid
                && runtime.child_start_time == self.runtime_child_start_time,
            RESUME_PROVIDER_AUTHORITY_INVALID
        );
        Ok(())
    }
}

pub(crate) fn provider_binding_stable_fields_match(
    planned: &AgentBinding,
    current: &AgentBinding,
) -> bool {
    current.schema_version == planned.schema_version
        && current.terminal_id == planned.terminal_id
        && current.kind == planned.kind
        && current.provider_session_id == planned.provider_session_id
        && current.executable == planned.executable
        && current.cwd == planned.cwd
        && current.evidence_path == planned.evidence_path
        && current.data_home == planned.data_home
        && current.detected_at_ms == planned.detected_at_ms
}

pub(crate) fn provider_resume_authority_digest(
    transaction: &ResumeTransaction,
    authority: &ProviderResumeAuthority,
) -> Result<String> {
    let body = ProviderResumeAuthorityDigestBody {
        schema_version: authority.schema_version,
        transaction_digest: transaction.transaction_digest.clone(),
        runtime_epoch: authority.runtime_epoch,
        runtime_child_pid: authority.runtime_child_pid,
        runtime_child_start_time: authority.runtime_child_start_time,
        binding: authority.binding.clone(),
        observed_at_ms: authority.observed_at_ms,
    };
    let bytes = serde_json::to_vec(&body).context(RESUME_TRANSACTION_SERIALIZE_CONTEXT)?;
    Ok(yolo_crypto::sha256_hex(&bytes))
}
