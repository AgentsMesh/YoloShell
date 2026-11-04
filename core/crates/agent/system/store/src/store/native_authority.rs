use super::{AgentBinding, AgentBindingStore};
use crate::agent::claude::generation::{
    AuthorityRuntimeIdentity,
    native::{ClaudeNativeGeneration, ClaudeNativeGenerationStore},
};
use crate::agent::codex::authority::{CodexAuthorityStatus, CodexBindingAuthority};
use yolo_agent_hook_generation::HookGenerationRecord;
use crate::agent::{AgentKind, AgentRecoveryAvailability, AgentState};
use crate::namespace_lease::NamespaceLease;
use crate::paths::identity::FileIdentity;
use anyhow::{Result, ensure};

#[path = "native_authority_selection.rs"]
mod selection;
pub(super) use selection::select;

#[path = "native_authority_verify.rs"]
mod verify;

pub(super) const INVALID: &str =
    "native agent authority changed or does not authorize this binding";

pub(super) fn claude_record_exists(
    store: &AgentBindingStore,
    namespace: &NamespaceLease,
) -> Result<bool> {
    let native = ClaudeNativeGenerationStore::from_binding_path(store.path())?;
    Ok(!namespace.file_is_missing(native.path())?)
}

#[derive(Clone)]
pub(super) enum NativeAuthority {
    Claude(Box<ClaudeNativeGeneration>, FileIdentity),
    Codex(Box<CodexBindingAuthority>, FileIdentity),
    GenericHook(Box<HookGenerationRecord>, FileIdentity),
}

impl NativeAuthority {
    pub(super) fn original(&self) -> &AgentBinding {
        match self {
            Self::Claude(record, _) => &record.binding,
            Self::Codex(record, _) => &record.binding,
            Self::GenericHook(record, _) => &record.binding,
        }
    }

    pub(super) fn matches_runtime(&self, runtime: AuthorityRuntimeIdentity) -> bool {
        match self {
            Self::Claude(record, _) => record.matches_runtime(runtime),
            Self::Codex(record, _) => record.matches_runtime(runtime),
            Self::GenericHook(record, _) => record.matches_runtime(runtime),
        }
    }

    /// The hook authority records the transcript identity it authorized. A
    /// missing identity means the provider had not created its transcript when
    /// the hook fired; some providers write it only on the first prompt.
    pub(super) fn transcript_absent(&self) -> bool {
        matches!(self, Self::GenericHook(record, _) if record.transcript_identity.is_none())
    }

    pub(super) fn binding(&self, runtime: AuthorityRuntimeIdentity) -> Result<AgentBinding> {
        ensure!(self.matches_runtime(runtime), INVALID);
        let mut binding = match self {
            Self::Claude(record, _) => {
                if !record.revoked {
                    ensure!(record.verify_resume_evidence()?, INVALID);
                }
                let relation = (!record.revoked)
                    .then(|| self.process_relation())
                    .transpose()?;
                let mut binding = record.recovery_binding(
                    relation == Some(crate::process::ProcessIdentityRelation::Matching),
                );
                if relation == Some(crate::process::ProcessIdentityRelation::Gone)
                    && record.final_session_end
                {
                    binding.state = AgentState::Exited;
                    binding.process_pid = None;
                    binding.process_start_time = None;
                }
                binding
            }
            Self::Codex(record, _) => {
                let mut binding = record
                    .recoverable_binding(runtime)?
                    .unwrap_or_else(|| record.binding.clone());
                if binding.recovery_availability == AgentRecoveryAvailability::Available
                    && binding.state != AgentState::Exited
                    && self.process_relation()? != crate::process::ProcessIdentityRelation::Matching
                {
                    binding.state = AgentState::Unknown;
                }
                binding
            }
            Self::GenericHook(record, _) => {
                if !record.revoked {
                    ensure!(record.verify_evidence()?, INVALID);
                }
                let relation = (!record.revoked)
                    .then(|| self.process_relation())
                    .transpose()?;
                let mut binding =
                    record.recovery_binding(relation == Some(crate::process::ProcessIdentityRelation::Matching));
                if relation == Some(crate::process::ProcessIdentityRelation::Gone)
                    && record.final_session_end
                {
                    binding.state = AgentState::Exited;
                    binding.process_pid = None;
                    binding.process_start_time = None;
                }
                binding
            }
        };
        if binding.recovery_availability == AgentRecoveryAvailability::Unavailable {
            binding.state = AgentState::Unknown;
        }
        Ok(binding)
    }

    pub(super) fn process_relation(&self) -> Result<crate::process::ProcessIdentityRelation> {
        let (boot, pid, start) = match self {
            Self::Claude(record, _) => (
                record.boot_id.as_str(),
                record.scope.agent.pid,
                record.scope.agent.start_time,
            ),
            Self::Codex(record, _) => {
                let process = record.provider_process();
                (
                    record
                        .resume_boot_id()
                        .ok_or_else(|| anyhow::anyhow!(INVALID))?,
                    process.pid,
                    process.start_time,
                )
            }
            Self::GenericHook(record, _) => (
                record.boot_id.as_str(),
                record.scope.agent.pid,
                record.scope.agent.start_time,
            ),
        };
        Ok(
            match crate::process::boot_identity_relation(boot, &crate::process::boot_id()) {
                crate::process::BootIdentityRelation::Same => {
                    crate::process::process_identity_relation(pid, start)
                }
                crate::process::BootIdentityRelation::Different => {
                    crate::process::ProcessIdentityRelation::Gone
                }
                crate::process::BootIdentityRelation::Unknown => anyhow::bail!(INVALID),
            },
        )
    }
}
