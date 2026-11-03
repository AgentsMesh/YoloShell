use super::authority_set::ActiveBindingPublication;
use super::constants::REVISION_INCREMENT;
use super::{ClaudeGenerationAuthority, GenerationEvent, GenerationRecoveryEvent};
use crate::agent::{AgentBinding, AgentState};
use crate::paths::identity::ObjectIdentity;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

pub(super) const TRANSITION_INVALID: &str = "Claude binding transition is invalid";
pub(super) const TRANSITION_TRANSCRIPT_CHANGED: &str =
    "Claude binding transition transcript changed";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PendingBindingTransition {
    #[serde(with = "crate::serde_uuid")]
    pub(super) invocation_id: Uuid,
    pub(super) publication_generation: u64,
    pub(super) predecessor_revision: u64,
    pub(super) predecessor_state: AgentState,
    pub(super) target_revision: u64,
    pub(super) target_state: AgentState,
    pub(super) predecessor_publication_generation: u64,
    pub(super) predecessor_publication: Option<ActiveBindingPublication>,
    pub(super) predecessor_authority: ClaudeGenerationAuthority,
    pub(super) predecessor: BindingTransitionEvidence,
    pub(super) target_binding: AgentBinding,
    pub(super) proof: BindingTransitionProof,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum BindingTransitionProof {
    Event {
        sequence: u64,
        event: GenerationEvent,
        transcript_identity: Option<ObjectIdentity>,
    },
    Recovery {
        events: Vec<GenerationRecoveryEvent>,
    },
    StateMutation {
        target_state: AgentState,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct BindingTransitionEvidence {
    pub(super) binding: AgentBinding,
    pub(super) transcript_path: PathBuf,
    pub(super) transcript_identity: ObjectIdentity,
}

impl PendingBindingTransition {
    #[cfg(any(test, feature = "adversarial-test-hooks"))]
    pub(crate) fn rewrite_boot_for_test(&mut self, current: &str, previous: &str) -> Result<()> {
        super::adversarial_test_support::rewrite_boot(
            &mut self.predecessor_authority.handshake_boot_id,
            current,
            previous,
        )?;
        Ok(())
    }

    pub(super) fn new(
        predecessor: &ClaudeGenerationAuthority,
        target: &ClaudeGenerationAuthority,
        publication_generation: u64,
        binding: AgentBinding,
        target_binding: AgentBinding,
        predecessor_publication_generation: u64,
        predecessor_publication: Option<ActiveBindingPublication>,
        proof: BindingTransitionProof,
    ) -> Result<Self> {
        Self::from_evidence(
            predecessor,
            target,
            publication_generation,
            BindingTransitionEvidence {
                binding,
                transcript_path: predecessor
                    .transcript_path
                    .clone()
                    .ok_or_else(|| anyhow::anyhow!(TRANSITION_INVALID))?,
                transcript_identity: predecessor
                    .transcript_identity
                    .ok_or_else(|| anyhow::anyhow!(TRANSITION_INVALID))?,
            },
            target_binding,
            predecessor_publication_generation,
            predecessor_publication,
            proof,
        )
    }

    fn from_evidence(
        predecessor: &ClaudeGenerationAuthority,
        target: &ClaudeGenerationAuthority,
        publication_generation: u64,
        evidence: BindingTransitionEvidence,
        target_binding: AgentBinding,
        predecessor_publication_generation: u64,
        predecessor_publication: Option<ActiveBindingPublication>,
        proof: BindingTransitionProof,
    ) -> Result<Self> {
        let transition = Self {
            invocation_id: target.invocation_id,
            publication_generation,
            predecessor_revision: predecessor.revision,
            predecessor_state: predecessor.state,
            target_revision: target.revision,
            target_state: target.state,
            predecessor_publication_generation,
            predecessor_publication,
            predecessor_authority: predecessor.clone(),
            predecessor: evidence,
            target_binding,
            proof,
        };
        ensure!(
            transition.proves_target(target, publication_generation),
            TRANSITION_INVALID
        );
        transition.verify_transcript()?;
        Ok(transition)
    }

    pub(super) fn matches_target(
        &self,
        authority: &ClaudeGenerationAuthority,
        publication_generation: u64,
    ) -> bool {
        self.invocation_id == authority.invocation_id
            && self.publication_generation == publication_generation
            && self.proves_target(authority, publication_generation)
    }

    fn proves_target(
        &self,
        authority: &ClaudeGenerationAuthority,
        publication_generation: u64,
    ) -> bool {
        self.target_revision == authority.revision
            && self.target_state == authority.state
            && self.predecessor_revision == self.predecessor_authority.revision
            && self.predecessor_state == self.predecessor_authority.state
            && self.publication_relation_is_valid()
            && authority.binding_revision.is_none()
            && self.proof_candidate() == Some(authority.clone())
            && super::binding_transition_validation::target_matches(self, authority)
            && publication_generation == self.publication_generation
    }

    pub(super) fn proves_target_shape(&self) -> bool {
        self.proof_candidate().is_some_and(|candidate| {
            candidate.invocation_id == self.invocation_id
                && candidate.revision == self.target_revision
                && candidate.state == self.target_state
                && candidate.binding_revision.is_none()
        })
    }

    pub(super) fn publication_relation_is_valid(&self) -> bool {
        self.predecessor_publication_generation
            .checked_add(REVISION_INCREMENT)
            == Some(self.publication_generation)
            && match self.predecessor_publication {
                Some(publication) => {
                    publication.publication_generation == self.predecessor_publication_generation
                        && publication.invocation_id == self.predecessor_authority.invocation_id
                        && publication.authority_revision == self.predecessor_authority.revision
                        && self.predecessor_authority.binding_revision
                            == Some(self.predecessor_authority.revision)
                }
                None => self.predecessor_authority.binding_revision.is_none(),
            }
    }

    fn proof_candidate(&self) -> Option<ClaudeGenerationAuthority> {
        super::binding_transition_proof::candidate(&self.predecessor_authority, &self.proof)
    }

    #[cfg(any(test, feature = "adversarial-test-hooks"))]
    pub(super) fn targets(&self, state: AgentState) -> bool {
        self.target_state == state
    }

    pub(super) fn accepts_binding(&self, binding: &AgentBinding) -> bool {
        binding == &self.predecessor.binding || binding == &self.target_binding
    }

    pub(super) fn target_binding(&self) -> &AgentBinding {
        &self.target_binding
    }
}
