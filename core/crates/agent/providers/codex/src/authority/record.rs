use yolo_agent_authority_contract::AuthorityRuntimeIdentity;
use crate::agent::hooks::{Generation, ProviderVersion};
use crate::agent::{AgentBinding, AgentKind, AgentRecoveryAvailability, AgentState};
use anyhow::{Result, ensure};
use serde::{Deserialize, Deserializer, Serialize};
use uuid::Uuid;

const AUTHORITY_INVALID: &str = "Codex binding authority is invalid";
const FIRST_ROUTE_REVISION: u64 = 1;

fn deserialize_required_option<'de, D, T>(
    deserializer: D,
) -> std::result::Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::deserialize(deserializer)
}

#[path = "record_constructor.rs"]
mod constructor;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodexAuthorityStatus {
    Active,
    Exhausted,
    Unavailable,
    Revoked,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CodexAuthorityVersion {
    pub route_revision: u64,
    #[serde(with = "crate::serde_uuid")]
    pub invocation_id: Uuid,
    pub generation: Generation,
    pub authority_revision: u64,
    pub sequence: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CodexBindingAuthority {
    schema_version: u16,
    #[serde(with = "crate::serde_uuid")]
    runtime_epoch: Uuid,
    worker_pid: u32,
    worker_start_time: u64,
    provider_pid: u32,
    provider_start_time: u64,
    provider_version: ProviderVersion,
    pub version: CodexAuthorityVersion,
    pub status: CodexAuthorityStatus,
    pub binding: AgentBinding,
    #[serde(deserialize_with = "deserialize_required_option")]
    resume_identity: Option<super::resume_identity::CodexResumeIdentity>,
}

impl CodexBindingAuthority {
    pub fn validate(&self) -> Result<()> {
        if let Some(resume) = self.resume_identity.as_ref() {
            resume.validate_for(&self.binding)?;
        }
        let active = matches!(
            (self.binding.state, self.binding.recovery_availability),
            (
                AgentState::Active
                    | AgentState::Running
                    | AgentState::WaitingForInput
                    | AgentState::WaitingForPermission
                    | AgentState::Exited,
                AgentRecoveryAvailability::Available
            )
        );
        let unavailable = self.binding.state == AgentState::Unknown
            && self.binding.recovery_availability == AgentRecoveryAvailability::Unavailable;
        let process_matches = if self.binding.state == AgentState::Exited {
            self.binding.process_pid.is_none() && self.binding.process_start_time.is_none()
        } else {
            self.binding.process_pid == Some(self.provider_pid)
                && self.binding.process_start_time == Some(self.provider_start_time)
        };
        let exited_resume_is_present =
            self.binding.state != AgentState::Exited || self.resume_identity.is_some();
        ensure!(
            self.schema_version == super::AUTHORITY_SCHEMA_VERSION
                && !self.runtime_epoch.is_nil()
                && self.worker_pid != 0
                && self.worker_start_time != 0
                && self.provider_pid != 0
                && self.provider_start_time != 0
                && self.provider_version == crate::codex_hooks::ACCEPTED_PROVIDER_VERSION
                && self.version.route_revision >= FIRST_ROUTE_REVISION
                && !self.version.invocation_id.is_nil()
                && self.version.generation.0 != 0
                && self.version.authority_revision != u64::default()
                && self.version.authority_revision != u64::MAX
                && self.binding.kind == AgentKind::Codex
                && self.binding.terminal_id != Uuid::nil()
                && process_matches
                && exited_resume_is_present
                && match self.status {
                    CodexAuthorityStatus::Active => active,
                    CodexAuthorityStatus::Exhausted
                    | CodexAuthorityStatus::Unavailable
                    | CodexAuthorityStatus::Revoked => {
                        unavailable
                    }
                },
            AUTHORITY_INVALID
        );
        Ok(())
    }

    pub fn matches_runtime(&self, runtime: AuthorityRuntimeIdentity) -> bool {
        self.runtime_epoch == runtime.runtime_epoch
            && self.worker_pid == runtime.worker.pid
            && self.worker_start_time == runtime.worker.start_time
    }

    pub(super) fn binding_matches_provider_process(&self, binding: &AgentBinding) -> bool {
        binding.process_pid == Some(self.provider_pid)
            && binding.process_start_time == Some(self.provider_start_time)
    }

    pub fn authorizes(&self, binding: &AgentBinding, runtime: AuthorityRuntimeIdentity) -> bool {
        self.validate().is_ok()
            && self.status == CodexAuthorityStatus::Active
            && &self.binding == binding
            && self.matches_runtime(runtime)
            && self
                .resume_identity
                .as_ref()
                .is_none_or(|resume| resume.verify().is_ok())
    }

    pub(super) fn is_canonical_terminal_successor_of(&self, current: &Self) -> bool {
        if current.status != CodexAuthorityStatus::Active
            || current.binding.state == AgentState::Exited
            || !matches!(
                self.status,
                CodexAuthorityStatus::Exhausted | CodexAuthorityStatus::Unavailable
            )
        {
            return false;
        }
        let mut expected = current.binding.clone();
        expected.state = AgentState::Unknown;
        expected.recovery_availability = AgentRecoveryAvailability::Unavailable;
        expected.process_pid = Some(current.provider_pid);
        expected.process_start_time = Some(current.provider_start_time);
        let updated_at_is_monotonic = self.binding.updated_at_ms >= expected.updated_at_ms;
        expected.updated_at_ms = self.binding.updated_at_ms;
        updated_at_is_monotonic && self.binding == expected
    }

    pub fn is_canonical_reactivation_of(&self, current: &Self) -> bool {
        if current.status != CodexAuthorityStatus::Unavailable
            || self.status != CodexAuthorityStatus::Active
            || self.provider_pid != current.provider_pid
            || self.provider_start_time != current.provider_start_time
            || self.provider_version != current.provider_version
            || !matches!(
                self.binding.state,
                AgentState::Active
                    | AgentState::Running
                    | AgentState::WaitingForInput
                    | AgentState::WaitingForPermission
                    | AgentState::Exited
            )
        {
            return false;
        }
        let mut expected = current.binding.clone();
        expected.state = self.binding.state;
        expected.recovery_availability = AgentRecoveryAvailability::Available;
        let active = self.binding.state != AgentState::Exited;
        expected.process_pid = active.then_some(current.provider_pid);
        expected.process_start_time = active.then_some(current.provider_start_time);
        let updated_at_is_monotonic = self.binding.updated_at_ms >= expected.updated_at_ms;
        expected.updated_at_ms = self.binding.updated_at_ms;
        updated_at_is_monotonic && self.binding == expected
    }
}
include!("record_terminal.rs");
include!("record_recovery.rs");
