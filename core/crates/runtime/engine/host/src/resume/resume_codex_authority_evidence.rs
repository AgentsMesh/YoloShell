use super::*;
use crate::paths::identity::FileIdentity;

use crate::agent::codex::authority::CodexBindingAuthority;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum ResumeCodexAuthorityEvidence {
    Parsed {
        authority: CodexBindingAuthority,
        file_identity: FileIdentity,
    },
    Raw {
        file_identity: FileIdentity,
    },
}

impl Host {
    pub(crate) fn capture_resume_codex_authority_evidence(
        &self,
        expected_session_id: Uuid,
        binding_store: &AgentBindingStore,
    ) -> Result<Option<ResumeCodexAuthorityEvidence>> {
        anyhow::ensure!(!expected_session_id.is_nil(), RESUME_BINDING_CHANGED);
        let store = binding_store.codex_authority_store()?;
        let Some(file_identity) = self
            .namespace
            .file(store.path())?
            .inspect(RESUME_BINDING_EVIDENCE_MAX_BYTES)?
        else {
            return Ok(None);
        };
        self.namespace.verify()?;
        let parsed = self
            .namespace
            .read_json::<CodexBindingAuthority>(store.path());
        self.namespace.verify()?;
        Ok(Some(match parsed {
            Ok(authority) if authority.validate().is_ok() => {
                anyhow::ensure!(
                    authority.binding.terminal_id == expected_session_id,
                    RESUME_BINDING_CHANGED
                );
                ResumeCodexAuthorityEvidence::Parsed {
                    authority,
                    file_identity,
                }
            }
            Ok(_) | Err(_) => ResumeCodexAuthorityEvidence::Raw { file_identity },
        }))
    }

    pub(crate) fn revoke_resume_codex_authority(
        &self,
        guard: &yolo_paths::durable_mutation::SessionDurableMutationGuard<'_>,
        transaction: &ResumeTransaction,
        marker: &ResumeBindingClearMarker,
        evidence: Option<&ResumeCodexAuthorityEvidence>,
    ) -> Result<()> {
        let terminal_id = transaction.terminal_id;
        let binding_store = self.agent_binding_store(terminal_id);
        let store = binding_store.codex_authority_store()?;
        match evidence {
            Some(ResumeCodexAuthorityEvidence::Parsed {
                authority,
                file_identity,
            }) => store.revoke_exact_in_namespace_with_commit_fence(
                &self.namespace,
                authority,
                *file_identity,
                || self.namespace.verify(),
            ),
            Some(ResumeCodexAuthorityEvidence::Raw { file_identity }) => {
                self.remove_raw_codex_authority(guard, &store, *file_identity, transaction, marker)
            }
            None => {
                anyhow::ensure!(
                    self.namespace.file_is_missing(store.path())?,
                    RESUME_BINDING_CHANGED
                );
                Ok(())
            }
        }
    }
}
