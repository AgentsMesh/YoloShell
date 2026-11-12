use super::*;
use crate::namespace_lease::NamespaceLease;
use crate::paths::identity::FileIdentity;
use anyhow::{Context, Result, ensure};
use std::path::{Path, PathBuf};

#[path = "store_publication.rs"]
mod publication;

#[derive(Clone, Debug)]
pub struct ClaudeNativeGenerationStore {
    pub(super) path: PathBuf,
    binding_path: PathBuf,
    pub(super) generation: super::super::ClaudeGenerationStore,
}

impl ClaudeNativeGenerationStore {
    pub fn from_binding_path(path: &Path) -> Result<Self> {
        Ok(Self {
            path: path.parent().context(INVALID)?.join(NATIVE_GENERATION_FILE),
            binding_path: path.to_path_buf(),
            generation: super::super::ClaudeGenerationStore::from_binding_path(path)?,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load_with_identity_in_namespace(
        &self,
        namespace: &NamespaceLease,
    ) -> Result<Option<(ClaudeNativeGeneration, FileIdentity)>> {
        Ok(self.load(namespace)?.and_then(|(state, identity)| {
            state
                .invocations
                .into_iter()
                .find(|item| item.invocation_id == state.active_invocation)
                .map(|record| (record, identity))
        }))
    }

    pub fn load_in_namespace(
        &self,
        namespace: &NamespaceLease,
    ) -> Result<Option<ClaudeNativeGeneration>> {
        Ok(self.load(namespace)?.and_then(|(state, _)| {
            state
                .invocations
                .into_iter()
                .find(|item| item.invocation_id == state.active_invocation)
        }))
    }

    pub fn load_scope_in_namespace(
        &self,
        namespace: &NamespaceLease,
        scope: ClaudeNativeScope,
    ) -> Result<Option<ClaudeNativeGeneration>> {
        Ok(self.load(namespace)?.and_then(|(state, _)| {
            state
                .invocations
                .into_iter()
                .find(|item| item.scope == scope)
        }))
    }

    pub fn apply_in_namespace(
        &self,
        namespace: &NamespaceLease,
        request: &ClaudeNativeRequest,
        worker: super::super::AuthorityProcessIdentity,
        boot_id: &str,
        evidence: &crate::claude::AuthoritativeClaudeEvidence,
        mut commit_fence: impl FnMut() -> Result<()>,
    ) -> Result<ClaudeNativeGeneration> {
        validation::event(request, evidence)?;
        let mutation =
            super::super::mutation::GenerationMutation::acquire(&self.generation, namespace)?;
        let loaded = self.load(namespace)?;
        let previous = loaded.as_ref().map(|(state, _)| state.clone());
        let identity = loaded.as_ref().map(|(_, identity)| *identity);
        let mut state = match loaded {
            Some((state, _)) => state,
            None => {
                let record = reducer::first(request, worker, boot_id, evidence)?;
                NativeGenerations {
                    schema_version: SCHEMA_VERSION,
                    active_invocation: record.invocation_id,
                    invocations: vec![record],
                }
            }
        };
        ensure!(
            state
                .invocations
                .iter()
                .all(|item| item.scope.terminal_id == request.scope.terminal_id),
            INVALID
        );
        let selected = if let Some(index) = state
            .invocations
            .iter()
            .position(|item| item.scope == request.scope)
        {
            let mut record = state.invocations[index].clone();
            ensure!(
                record.worker == worker && record.boot_id == boot_id,
                INVALID
            );
            if reducer::apply(&mut record, request, evidence)? {
                super::session_origin::inherit(&mut record, request, &state.invocations);
                state.invocations[index] = record;
            }
            index
        } else {
            ensure!(
                state.invocations.len() < super::super::constants::MAX_INVOCATION_AUTHORITIES,
                CAPACITY
            );
            let mut record = reducer::first(request, worker, boot_id, evidence)?;
            super::session_origin::inherit(&mut record, request, &state.invocations);
            state.invocations.push(record);
            state.invocations.len() - INCREMENT as usize
        };
        let record = state.invocations[selected].clone();
        state.active_invocation = record.invocation_id;
        state.validate()?;
        if previous.as_ref() == Some(&state) {
            mutation.require_current()?;
            ensure!(
                evidence.verify()? && record.verify_resume_evidence()?,
                INVALID
            );
            commit_fence()?;
            mutation.require_current()?;
            ensure!(
                self.load(namespace)?
                    .as_ref()
                    .is_some_and(|(current, current_identity)| current == &state
                        && Some(*current_identity) == identity)
                    && evidence.verify()?
                    && record.verify_resume_evidence()?,
                INVALID
            );
            // An exact accepted replay changes neither authority nor binding identity.
            return Ok(record);
        }
        mutation.write_path(namespace, &self.path, identity, &state, || {
            ensure!(
                evidence.verify()? && record.verify_resume_evidence()?,
                INVALID
            );
            commit_fence()
        })?;
        Ok(record)
    }

    pub(super) fn load(
        &self,
        namespace: &NamespaceLease,
    ) -> Result<Option<(NativeGenerations, FileIdentity)>> {
        namespace.verify()?;
        if namespace.file_is_missing(&self.path)? {
            return Ok(None);
        }
        let (mut state, identity): (NativeGenerations, _) =
            namespace.read_json_with_identity(&self.path)?;
        state.migrate_replay_schema();
        state.validate()?;
        Ok(Some((state, identity)))
    }
}
