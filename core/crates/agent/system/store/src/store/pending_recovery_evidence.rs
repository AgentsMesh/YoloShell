use crate::agent::AgentState;
use crate::agent::claude::ClaudeAdapter;
use crate::agent::claude::generation::ClaudeGenerationAuthority;
use crate::claude_hooks::outbox::PendingRecovery;
use anyhow::{Context, Result, ensure};

pub(super) struct RecoveryEvidence {
    fence: crate::agent::claude::AuthoritativeClaudeEvidence,
}

pub(super) fn inspect(
    authority: &ClaudeGenerationAuthority,
    pending: &PendingRecovery,
) -> Result<RecoveryEvidence> {
    let transcript = authority
        .transcript_path
        .as_deref()
        .context(crate::agent::constants::recovery::TRANSCRIPT_MISSING)?;
    let fence = ClaudeAdapter
        .inspect_authoritative_evidence_with_hook(
            transcript,
            &authority.cwd,
            &authority.data_home,
            || {},
        )?
        .context(crate::agent::constants::recovery::EVIDENCE_MISSING)?;
    let evidence = RecoveryEvidence { fence };
    evidence.verify(authority)?;
    let last_transcript = pending
        .events
        .iter()
        .rev()
        .find_map(|event| event.transcript_identity);
    ensure!(
        last_transcript.is_none_or(|identity| identity == evidence.fence.identity()),
        crate::agent::constants::recovery::EVIDENCE_CHANGED
    );
    Ok(evidence)
}

impl RecoveryEvidence {
    pub(super) fn verify(&self, authority: &ClaudeGenerationAuthority) -> Result<()> {
        let observed = self.fence.evidence();
        ensure!(
            self.fence.verify()?
                && authority.transcript_identity == Some(self.fence.identity())
                && authority.transcript_path.as_deref() == Some(observed.path.as_path())
                && authority.provider_session_id == observed.provider_session_id
                && authority.cwd == observed.cwd
                && authority.data_home == observed.data_home
                && matches!(
                    authority.state,
                    AgentState::Running | AgentState::WaitingForInput
                ),
            crate::agent::constants::recovery::EVIDENCE_CHANGED
        );
        Ok(())
    }
}
