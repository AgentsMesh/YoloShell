use anyhow::{Result, ensure};

mod build;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;
use yolo_agent_authority_contract::{AuthorityProcessIdentity, AuthorityRuntimeIdentity};
use yolo_agent_contract::{AgentBinding, AgentKind, AgentRecoveryAvailability, AgentState};
use yolo_hooks_runtime::{
    ExpectedIdentity, Generation, HookIdentity, HookProviderKind, ProviderVersion, Reducer,
    ReducerSnapshot,
};
use yolo_paths::identity::ObjectIdentity;

pub const GENERATION_AUTHORITY_FILE: &str = "hook-generation-authority.json";
const SCHEMA_VERSION: u32 = 1;
const INITIAL_REVISION: u64 = 1;
const INCREMENT: u64 = 1;
pub(crate) const INVALID: &str =
    "hook generation authority is invalid, changed or does not authorize this agent";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenerationScope {
    #[serde(with = "yolo_crypto::serde_uuid")]
    pub terminal_id: Uuid,
    #[serde(with = "yolo_crypto::serde_uuid")]
    pub runtime_epoch: Uuid,
    pub agent: AuthorityProcessIdentity,
}

impl GenerationScope {
    pub fn is_valid(self) -> bool {
        !self.terminal_id.is_nil()
            && !self.runtime_epoch.is_nil()
            && self.agent.pid != u32::default()
            && self.agent.start_time != u64::default()
    }

    pub fn process(self) -> yolo_hooks_runtime::ProcessIdentity {
        yolo_hooks_runtime::ProcessIdentity::new(self.agent.pid, self.agent.start_time)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HookGenerationRecord {
    pub schema_version: u32,
    pub scope: GenerationScope,
    pub provider: HookProviderKind,
    pub provider_version: ProviderVersion,
    pub worker: AuthorityProcessIdentity,
    pub boot_id: String,
    #[serde(with = "yolo_crypto::serde_uuid")]
    pub invocation_id: Uuid,
    #[serde(with = "yolo_crypto::serde_uuid")]
    pub hook_token: Uuid,
    pub revision: u64,
    pub generation: u64,
    pub last_event_sequence: u64,
    pub provider_session_id: String,
    pub binding: AgentBinding,
    pub state: AgentState,
    pub final_session_end: bool,
    pub revoked: bool,
    pub transcript_identity: Option<ObjectIdentity>,
    pub cwd_identity: ObjectIdentity,
    pub data_home_identity: ObjectIdentity,
    pub snapshot: ReducerSnapshot,
}

impl PartialEq for HookGenerationRecord {
    fn eq(&self, other: &Self) -> bool {
        serde_json::to_vec(self).ok() == serde_json::to_vec(other).ok()
    }
}

impl HookGenerationRecord {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.schema_version == SCHEMA_VERSION
                && self.scope.is_valid()
                && !self.invocation_id.is_nil()
                && !self.hook_token.is_nil()
                && self.generation != u64::default()
                && self.revision >= INITIAL_REVISION
                && !self.provider_session_id.is_empty()
                && self.binding.terminal_id == self.scope.terminal_id
                && self.binding.provider_session_id == self.provider_session_id,
            INVALID
        );
        let reducer = self.reducer()?;
        ensure!(
            reducer.state().provider == self.provider
                && reducer.state().provider_version == self.provider_version
                && (self.revoked
                    || (self.state == super::apply::activity(reducer.state())
                        && self.final_session_end == reducer.state().final_hook_seen)),
            INVALID
        );
        Ok(())
    }

    pub fn expected_identity(&self) -> ExpectedIdentity {
        HookIdentity {
            terminal_id: self.scope.terminal_id,
            runtime_epoch: self.scope.runtime_epoch,
            invocation_id: self.invocation_id,
            hook_token: self.hook_token,
            process: self.scope.process(),
            generation: Generation(self.generation),
        }
    }

    pub fn reducer(&self) -> Result<Reducer> {
        Ok(Reducer::restore(
            self.snapshot.clone(),
            self.expected_identity(),
            self.provider.clone(),
            self.provider_version,
        )?)
    }

    pub fn matches_runtime(&self, runtime: AuthorityRuntimeIdentity) -> bool {
        self.scope.runtime_epoch == runtime.runtime_epoch && self.worker == runtime.worker
    }

    pub fn recovery_binding(&self, activity_is_current: bool) -> AgentBinding {
        let mut binding = self.binding.clone();
        binding.state = if activity_is_current {
            self.state
        } else {
            AgentState::Unknown
        };
        binding.recovery_availability = if self.revoked || self.transcript_identity.is_none() {
            AgentRecoveryAvailability::Unavailable
        } else {
            AgentRecoveryAvailability::Available
        };
        if binding.state == AgentState::Exited {
            binding.process_pid = None;
            binding.process_start_time = None;
        }
        binding
    }

    pub fn authorizes_resume(&self, binding: &AgentBinding) -> bool {
        !self.revoked
            && self.transcript_identity.is_some()
            && binding.terminal_id == self.scope.terminal_id
            && binding.kind == self.binding.kind
            && binding.provider_session_id == self.binding.provider_session_id
            && binding.cwd == self.binding.cwd
            && binding.data_home == self.binding.data_home
            && binding.evidence_path == self.binding.evidence_path
            && self.verify_evidence().unwrap_or(false)
    }

    pub fn verify_evidence(&self) -> Result<bool> {
        super::evidence::verify(self)
    }

}

pub(crate) fn increment(value: u64) -> Result<u64> {
    value.checked_add(INCREMENT).ok_or_else(|| anyhow::anyhow!(INVALID))
}

pub(crate) fn parent(path: &Path) -> Result<PathBuf> {
    path.parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| anyhow::anyhow!(INVALID))
}
