use super::binding_transition::PendingBindingTransition;
use super::constants::*;
use super::{AuthorityAuthentication, ClaudeGenerationAuthority, GenerationBegin};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};
use uuid::Uuid;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaudeInvocationAuthorities {
    pub(super) schema_version: u32,
    #[serde(with = "crate::serde_uuid")]
    pub(super) terminal_id: Uuid,
    #[serde(with = "crate::serde_uuid")]
    pub(super) active_invocation_id: Uuid,
    pub(super) publication_generation: u64,
    pub(super) binding_publication: Option<ActiveBindingPublication>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) pending_binding_transition: Option<PendingBindingTransition>,
    pub(super) authorities: VecDeque<ClaudeGenerationAuthority>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ActiveBindingPublication {
    #[serde(with = "crate::serde_uuid")]
    pub invocation_id: Uuid,
    pub(super) publication_generation: u64,
    pub authority_revision: u64,
}

impl ClaudeInvocationAuthorities {
    pub(super) fn new(authority: ClaudeGenerationAuthority) -> Self {
        Self {
            schema_version: AUTHORITY_SET_SCHEMA_VERSION,
            terminal_id: authority.terminal_id,
            active_invocation_id: authority.invocation_id,
            publication_generation: INITIAL_PUBLICATION_GENERATION,
            binding_publication: None,
            pending_binding_transition: None,
            authorities: VecDeque::from([authority]),
        }
    }

    pub(super) fn validate(&self) -> Result<()> {
        let mut invocations = HashSet::new();
        let active = self.authority(self.active_invocation_id);
        let publication_valid = match (self.binding_publication, active) {
            (None, Some(authority)) => authority.binding_revision.is_none(),
            (Some(publication), Some(authority)) => {
                publication.invocation_id == self.active_invocation_id
                    && publication.publication_generation == self.publication_generation
                    && publication.authority_revision == authority.revision
                    && authority.binding_revision == Some(authority.revision)
            }
            (_, None) => false,
        };
        let transition_valid = match (&self.pending_binding_transition, active) {
            (None, _) => true,
            (Some(transition), Some(authority)) => {
                self.binding_publication.is_none()
                    && transition.validate_shape().is_ok()
                    && transition.matches_target(authority, self.publication_generation)
            }
            (Some(_), None) => false,
        };
        ensure!(
            self.schema_version == AUTHORITY_SET_SCHEMA_VERSION
                && !self.terminal_id.is_nil()
                && !self.active_invocation_id.is_nil()
                && self.publication_generation >= INITIAL_PUBLICATION_GENERATION
                && !self.authorities.is_empty()
                && self.authorities.len() <= MAX_INVOCATION_AUTHORITIES
                && self.authorities.iter().all(|authority| {
                    authority.terminal_id == self.terminal_id
                        && invocations.insert(authority.invocation_id)
                        && super::record::validate_loaded(authority).is_ok()
                })
                && invocations.contains(&self.active_invocation_id)
                && publication_valid
                && transition_valid,
            AUTHORITY_SET_INVALID
        );
        Ok(())
    }

    pub(super) fn active(&self) -> Result<&ClaudeGenerationAuthority> {
        self.authority(self.active_invocation_id)
            .ok_or_else(|| anyhow::anyhow!(AUTHORITY_SET_INVALID))
    }

    pub(super) fn authority(&self, invocation_id: Uuid) -> Option<&ClaudeGenerationAuthority> {
        self.authorities
            .iter()
            .find(|authority| authority.invocation_id == invocation_id)
    }

    pub(super) fn matching_begin(
        &self,
        begin: &GenerationBegin,
    ) -> Option<&ClaudeGenerationAuthority> {
        self.authorities
            .iter()
            .find(|authority| super::record::matches_begin(authority, begin))
    }

    pub(super) fn authenticated(
        &self,
        authentication: AuthorityAuthentication,
    ) -> Option<&ClaudeGenerationAuthority> {
        self.authority(authentication.invocation_id)
            .filter(|authority| super::record::authenticated(authority, authentication))
    }

    pub(super) fn replace(
        &mut self,
        expected: &ClaudeGenerationAuthority,
        replacement: ClaudeGenerationAuthority,
    ) -> Result<()> {
        let current = self
            .authorities
            .iter_mut()
            .find(|authority| authority.invocation_id == expected.invocation_id)
            .ok_or_else(|| anyhow::anyhow!(AUTHORITY_MISSING))?;
        ensure!(*current == *expected, AUTHORITY_SNAPSHOT_MISMATCH);
        *current = replacement;
        Ok(())
    }

    pub(super) fn activate(&mut self, invocation_id: Uuid) -> Result<bool> {
        ensure!(self.authority(invocation_id).is_some(), AUTHORITY_MISSING);
        let changed = self.active_invocation_id != invocation_id;
        if changed {
            ensure!(
                self.pending_binding_transition.is_none(),
                AUTHORITY_PUBLICATION_STATE_INVALID
            );
            self.bump_publication()?;
            self.active_invocation_id = invocation_id;
            self.binding_publication = None;
            self.authorities
                .iter_mut()
                .find(|authority| authority.invocation_id == invocation_id)
                .expect(AUTHORITY_MISSING)
                .binding_revision = None;
        }
        Ok(changed)
    }

    pub(super) fn bump_publication(&mut self) -> Result<()> {
        ensure!(
            self.pending_binding_transition.is_none(),
            AUTHORITY_PUBLICATION_STATE_INVALID
        );
        self.bump_publication_preserving_transition()?;
        Ok(())
    }

    pub(super) fn bump_publication_preserving_transition(&mut self) -> Result<()> {
        self.publication_generation = self
            .publication_generation
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!(AUTHORITY_PUBLICATION_GENERATION_OVERFLOW))?;
        self.binding_publication = None;
        Ok(())
    }
}
