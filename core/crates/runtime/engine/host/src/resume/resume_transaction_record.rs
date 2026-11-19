use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ResumeTransaction {
    pub(crate) schema_version: u16,
    #[serde(with = "crate::serde_uuid")]
    pub(crate) terminal_id: Uuid,
    #[serde(with = "crate::serde_uuid")]
    pub(crate) previous_runtime_epoch: Uuid,
    #[serde(with = "crate::serde_uuid")]
    pub(crate) planned_runtime_epoch: Uuid,
    #[serde(with = "crate::serde_uuid")]
    pub(crate) worker_id: Uuid,
    pub(crate) intent: ResumeIntent,
    pub(crate) target: ResumeTarget,
    pub(crate) launch: WorkerLaunchSpec,
    pub(crate) launch_digest: String,
    pub(crate) transaction_digest: String,
    pub(crate) provider_completion: Option<ProviderResumeAuthority>,
    pub(crate) created_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum ResumeTarget {
    Shell {
        binding: ResumeBindingDisposition,
    },
    Provider {
        agent_kind: crate::agent::AgentKind,
        provider_session_id: String,
        binding: AgentBinding,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum ResumeBindingDisposition {
    Preserve,
    Clear {
        #[serde(with = "crate::serde_uuid")]
        operation_id: Uuid,
        previous: Option<ResumeBindingEvidence>,
        codex_authority: Option<ResumeCodexAuthorityEvidence>,
    },
}

impl ResumeTarget {
    pub(crate) fn launch_command(&self, manifest: &TerminalManifest) -> ShellCommand {
        let cwd = match self {
            Self::Provider { binding, .. } => Path::new(&binding.cwd),
            Self::Shell { .. } => Path::new(&manifest.cwd),
        };
        shell_command(cwd)
    }

    pub(crate) fn from_binding(binding: &ResumeBinding) -> Self {
        match binding {
            ResumeBinding::Shell(binding) => Self::Shell {
                binding: binding.clone(),
            },
            ResumeBinding::Provider(binding) => Self::Provider {
                agent_kind: binding.kind.clone(),
                provider_session_id: binding.provider_session_id.clone(),
                binding: binding.clone(),
            },
        }
    }

    pub(crate) const fn completed_state(&self) -> RecoveryState {
        match self {
            Self::Shell { .. } => RecoveryState::Recreated,
            Self::Provider { .. } => RecoveryState::Resumed,
        }
    }

    pub(crate) fn validate_for(&self, terminal_id: Uuid, intent: ResumeIntent) -> Result<()> {
        match self {
            Self::Shell {
                binding: ResumeBindingDisposition::Clear { operation_id, .. },
            } => {
                anyhow::ensure!(!operation_id.is_nil(), RESUME_TRANSACTION_INVALID);
            }
            Self::Shell {
                binding: ResumeBindingDisposition::Preserve,
            } => {}
            Self::Provider {
                agent_kind,
                provider_session_id,
                binding,
            } => {
                anyhow::ensure!(
                    !provider_session_id.is_empty() && provider_session_id.trim() == provider_session_id,
                    RESUME_TARGET_SESSION_EMPTY
                );
                anyhow::ensure!(
                    binding.terminal_id == terminal_id
                        && &binding.kind == agent_kind
                        && binding.provider_session_id == *provider_session_id
                        && Path::new(&binding.cwd).is_absolute()
                        && intent.policy(binding) == ResumePolicy::Provider,
                    RESUME_TRANSACTION_INVALID
                );
            }
        }
        Ok(())
    }
}
