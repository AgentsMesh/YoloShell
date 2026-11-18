use super::record::{
    GENERATION_AUTHORITY_FILE, GenerationScope, HookGenerationRecord, INVALID, increment, parent,
};
use super::GenerationEvidence;
use anyhow::{Context, Result, ensure};

mod apply;
use std::path::{Path, PathBuf};
use uuid::Uuid;
use yolo_agent_authority_contract::AuthorityProcessIdentity;
use yolo_agent_contract::AgentBinding;
use yolo_hooks_runtime::{
    EventKind, HookMetadata, HookProviderKind, ProviderVersion, SessionStartSource,
};
use yolo_paths::identity::FileIdentity;
use yolo_paths::namespace_lease::NamespaceLease;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GenerationRequest {
    pub scope: GenerationScope,
    pub event_sequence: u64,
    pub event_id: Uuid,
    pub provider: HookProviderKind,
    pub provider_version: ProviderVersion,
    pub provider_session_id: String,
    pub turn_id: Option<String>,
    pub agent_id: Option<String>,
    pub tool_name: Option<String>,
    pub tool_use_id: Option<String>,
    pub stop_hook_active: Option<bool>,
    pub metadata: HookMetadata,
    pub event: EventKind,
}

impl GenerationRequest {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.scope.is_valid()
                && self.event_sequence >= yolo_hooks_runtime::FIRST_SEQUENCE
                && !self.event_id.is_nil()
                && !self.provider_session_id.is_empty()
                && self.provider_session_id.len() <= yolo_hooks_runtime::MAX_HOOK_IDENTIFIER_BYTES
                && self.metadata.is_valid(),
            INVALID
        );
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct HookGenerationStore {
    binding_path: PathBuf,
    path: PathBuf,
}

impl HookGenerationStore {
    pub fn from_binding_path(binding_path: &Path) -> Result<Self> {
        Ok(Self {
            binding_path: binding_path.to_path_buf(),
            path: parent(binding_path)?.join(GENERATION_AUTHORITY_FILE),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn binding_path(&self) -> &Path {
        &self.binding_path
    }

    pub fn load_in_namespace(
        &self,
        namespace: &NamespaceLease,
    ) -> Result<Option<HookGenerationRecord>> {
        Ok(self
            .load_with_identity_in_namespace(namespace)?
            .map(|(record, _)| record))
    }

    pub fn load_with_identity_in_namespace(
        &self,
        namespace: &NamespaceLease,
    ) -> Result<Option<(HookGenerationRecord, FileIdentity)>> {
        namespace.verify()?;
        if namespace.file_is_missing(&self.path)? {
            return Ok(None);
        }
        let (record, identity): (HookGenerationRecord, _) =
            namespace.read_json_with_identity(&self.path)?;
        record.validate()?;
        Ok(Some((record, identity)))
    }

    fn write_authority(
        &self,
        namespace: &NamespaceLease,
        expected: Option<FileIdentity>,
        record: &HookGenerationRecord,
        evidence: &GenerationEvidence,
        mut commit_fence: impl FnMut() -> Result<()>,
    ) -> Result<()> {
        namespace.write_json_expected_with_commit_fence(
            &self.path,
            expected,
            record,
            || {
                ensure!(
                    record.verify_evidence()? && evidence.verify_own()?,
                    INVALID
                );
                commit_fence()
            },
        )?;
        ensure!(
            self.load_in_namespace(namespace)?.as_ref() == Some(record),
            INVALID
        );
        Ok(())
    }

    pub fn record_matches_binding(
        &self,
        namespace: &NamespaceLease,
        binding: &AgentBinding,
    ) -> Result<bool> {
        let Some(record) = self.load_in_namespace(namespace)? else {
            return Ok(false);
        };
        Ok(!record.revoked
            && record.binding.terminal_id == binding.terminal_id
            && record.binding.kind == binding.kind
            && record.binding.provider_session_id == binding.provider_session_id)
    }
}

