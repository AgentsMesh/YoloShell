use super::*;

pub(crate) const RESUME_BINDING_MARKER_INVALID: &str = "resume binding clear marker is invalid";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ResumeBindingClearPhase {
    Pending,
    Cleared,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ResumeBindingClearMarker {
    pub(crate) schema_version: u16,
    #[serde(with = "crate::serde_uuid")]
    pub(crate) terminal_id: Uuid,
    #[serde(with = "crate::serde_uuid")]
    pub(crate) operation_id: Uuid,
    #[serde(with = "crate::serde_uuid")]
    pub(crate) planned_runtime_epoch: Uuid,
    pub(crate) transaction_digest: String,
    pub(crate) previous: Option<ResumeBindingEvidence>,
    pub(crate) codex_authority: Option<ResumeCodexAuthorityEvidence>,
    pub(crate) phase: ResumeBindingClearPhase,
}

impl Host {
    pub(crate) fn apply_resume_binding_disposition(
        &self,
        transaction: &ResumeTransaction,
    ) -> Result<()> {
        let Some((_, _, _)) = binding_to_clear(transaction) else {
            return self.ensure_binding_marker_missing(transaction.terminal_id);
        };
        let marker: ResumeBindingClearMarker =
            self.read_namespace_json(&self.paths.resume_binding_clear(transaction.terminal_id))?;
        marker.validate_for(transaction)?;
        anyhow::ensure!(
            marker.phase == ResumeBindingClearPhase::Cleared,
            RESUME_BINDING_MARKER_INVALID
        );
        Ok(())
    }

    pub(crate) fn complete_resume_binding_disposition(
        &self,
        transaction: &ResumeTransaction,
    ) -> Result<()> {
        let Some((_, _, _)) = binding_to_clear(transaction) else {
            return self.ensure_binding_marker_missing(transaction.terminal_id);
        };
        let path = self.paths.resume_binding_clear(transaction.terminal_id);
        let Some(marker) = self.read_optional_resume_json::<ResumeBindingClearMarker>(&path)?
        else {
            return Ok(());
        };
        marker.validate_for(transaction)?;
        anyhow::ensure!(
            marker.phase == ResumeBindingClearPhase::Cleared,
            RESUME_BINDING_MARKER_INVALID
        );
        self.namespace.remove_json_if_matches(&path, &marker)
    }

    pub(crate) fn roll_resume_binding_marker(
        &self,
        marker: ResumeBindingClearMarker,
        transaction: &ResumeTransaction,
    ) -> Result<ResumeBindingClearMarker> {
        marker.validate_rollover_to(transaction)?;
        let rolled = binding_clear_marker(transaction, ResumeBindingClearPhase::Cleared)?;
        self.replace_resume_binding_marker(&marker, &rolled)?;
        Ok(rolled)
    }

    pub(crate) fn clear_previous_binding(
        &self,
        terminal_id: Uuid,
        previous: Option<&ResumeBindingEvidence>,
    ) -> Result<()> {
        let store = self.agent_binding_store(terminal_id);
        if self.namespace.file_is_missing(store.path())? {
            return Ok(());
        }
        let Some(previous) = previous else {
            anyhow::bail!(RESUME_BINDING_CHANGED);
        };
        self.clear_resume_binding_evidence(store.path(), previous)
    }

    pub(crate) fn ensure_binding_marker_missing(&self, terminal_id: Uuid) -> Result<()> {
        anyhow::ensure!(
            self.namespace
                .file_is_missing(&self.paths.resume_binding_clear(terminal_id))?,
            RESUME_BINDING_MARKER_INVALID
        );
        Ok(())
    }
}

impl ResumeBindingClearMarker {
    pub(crate) fn validate_for(&self, transaction: &ResumeTransaction) -> Result<()> {
        transaction.validate_for(transaction.terminal_id)?;
        let Some((operation_id, previous, codex_authority)) = binding_to_clear(transaction) else {
            anyhow::bail!(RESUME_BINDING_MARKER_INVALID);
        };
        anyhow::ensure!(
            self.schema_version == crate::constants::RESUME_BINDING_CLEAR_SCHEMA_VERSION
                && self.terminal_id == transaction.terminal_id
                && !self.terminal_id.is_nil()
                && self.operation_id == operation_id
                && self.planned_runtime_epoch == transaction.planned_runtime_epoch
                && self.transaction_digest == transaction.transaction_digest
                && &self.previous == previous
                && &self.codex_authority == codex_authority,
            RESUME_BINDING_MARKER_INVALID
        );
        Ok(())
    }

    pub(crate) fn validate_rollover_to(&self, transaction: &ResumeTransaction) -> Result<()> {
        transaction.validate_for(transaction.terminal_id)?;
        let Some((operation_id, previous, codex_authority)) = binding_to_clear(transaction) else {
            anyhow::bail!(RESUME_BINDING_MARKER_INVALID);
        };
        anyhow::ensure!(
            self.schema_version == crate::constants::RESUME_BINDING_CLEAR_SCHEMA_VERSION
                && self.terminal_id == transaction.terminal_id
                && self.operation_id == operation_id
                && self.planned_runtime_epoch == transaction.previous_runtime_epoch
                && self.transaction_digest != transaction.transaction_digest
                && valid_lower_hex(&self.transaction_digest, SHA256_HEX_LENGTH)
                && &self.previous == previous
                && &self.codex_authority == codex_authority
                && self.phase == ResumeBindingClearPhase::Cleared,
            RESUME_BINDING_MARKER_INVALID
        );
        Ok(())
    }
}

pub(crate) fn binding_to_clear(
    transaction: &ResumeTransaction,
) -> Option<(
    Uuid,
    &Option<ResumeBindingEvidence>,
    &Option<ResumeCodexAuthorityEvidence>,
)> {
    match &transaction.target {
        ResumeTarget::Shell {
            binding:
                ResumeBindingDisposition::Clear {
                    operation_id,
                    previous,
                    codex_authority,
                },
        } => Some((*operation_id, previous, codex_authority)),
        _ => None,
    }
}

pub(crate) fn binding_clear_marker(
    transaction: &ResumeTransaction,
    phase: ResumeBindingClearPhase,
) -> Result<ResumeBindingClearMarker> {
    let (operation_id, previous, codex_authority) =
        binding_to_clear(transaction).context(RESUME_BINDING_MARKER_INVALID)?;
    Ok(ResumeBindingClearMarker {
        schema_version: crate::constants::RESUME_BINDING_CLEAR_SCHEMA_VERSION,
        terminal_id: transaction.terminal_id,
        operation_id,
        planned_runtime_epoch: transaction.planned_runtime_epoch,
        transaction_digest: transaction.transaction_digest.clone(),
        previous: previous.clone(),
        codex_authority: codex_authority.clone(),
        phase,
    })
}
