use super::{AgentBinding, AgentRegistry, DetectionContext};
use crate::agent::adapter::SessionGenerationPolicy;
use crate::agent::{AgentRecoveryAvailability, AgentState};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use yolo_agent_providers::ProviderRuntimeRegistry;

#[path = "authority.rs"]
mod authority;
#[path = "claude_recovery_authority.rs"]
mod claude_recovery_authority;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/codex_checkpoint_tests.rs"]
mod codex_checkpoint_tests;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/codex_checkpoint_transition_tests.rs"]
mod codex_checkpoint_transition_tests;
#[path = "codex_recovery_authority.rs"]
mod codex_recovery_authority;
#[path = "detected_reconciliation.rs"]
mod detected_reconciliation;
#[path = "foreground_authority.rs"]
mod foreground_authority;
mod generic_recovery_authority;
#[path = "historical_exit.rs"]
mod historical_exit;
#[path = "hook_reconciliation.rs"]
mod hook_reconciliation;
mod native_authority;
mod native_authority_runtime;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/native_exit_tests.rs"]
mod native_exit_tests;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/native_fence_tests.rs"]
mod native_fence_tests;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/native_projection_lock_tests.rs"]
mod native_projection_lock_tests;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/native_generic_recovery_tests.rs"]
mod native_generic_recovery_tests;
mod native_read;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/native_recovery_tests.rs"]
mod native_recovery_tests;
mod native_revocation;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/native_subject_matrix_tests.rs"]
mod native_subject_matrix_tests;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../test_support/native_fixture.rs"]
mod native_test_support;
#[path = "observation.rs"]
mod observation;
#[path = "pending_recovery.rs"]
mod pending_recovery;
#[path = "pending_recovery_binding.rs"]
mod pending_recovery_binding;
#[path = "pending_recovery_evidence.rs"]
mod pending_recovery_evidence;
#[path = "pending_recovery_orphans.rs"]
mod pending_recovery_orphans;
#[path = "published_authority.rs"]
mod published_authority;
#[path = "recovery_authority.rs"]
mod recovery_authority;

use recovery_authority::BindingRecoveryAuthority;

#[derive(Debug, Clone)]
pub struct AgentBindingStore {
    path: PathBuf,
    registry: Arc<AgentRegistry>,
    provider_runtime: Arc<ProviderRuntimeRegistry>,
}

impl AgentBindingStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self::with_provider_runtime(
            path,
            Arc::new(yolo_agent_providers::builtin_provider_runtime()),
        )
    }

    pub fn with_provider_runtime(
        path: impl Into<PathBuf>,
        provider_runtime: Arc<ProviderRuntimeRegistry>,
    ) -> Self {
        let registry = Arc::new(yolo_agent_providers::agent_registry_from_runtime(
            &provider_runtime,
        ));
        Self {
            path: path.into(),
            registry,
            provider_runtime,
        }
    }

    pub fn registry(&self) -> &AgentRegistry {
        &self.registry
    }

    pub fn provider_runtime(&self) -> &ProviderRuntimeRegistry {
        &self.provider_runtime
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(&self, context: &DetectionContext) -> Result<Option<AgentBinding>> {
        let Some(binding) = self.binding_file().read()? else {
            return Ok(None);
        };
        let binding = recovery_safe(binding, &self.registry, None);
        self.registry.validate_binding(&binding, context)?;
        Ok(Some(binding))
    }

    pub fn save(&self, binding: &AgentBinding) -> Result<()> {
        self.binding_file().write(binding)
    }

    pub fn clear(&self) -> Result<()> {
        self.binding_file().clear()
    }

    pub fn codex_authority_store(
        &self,
    ) -> Result<super::codex::authority::CodexBindingAuthorityStore> {
        super::codex::authority::CodexBindingAuthorityStore::from_binding_path(&self.path)
    }

    fn binding_file(&self) -> yolo_agent_binding_io::AgentBindingFile {
        yolo_agent_binding_io::AgentBindingFile::new(&self.path)
    }
}

fn is_missing(error: &anyhow::Error) -> bool {
    error
        .chain()
        .find_map(|source| source.downcast_ref::<std::io::Error>())
        .is_some_and(|source| source.kind() == std::io::ErrorKind::NotFound)
}

fn recovery_safe(
    mut binding: AgentBinding,
    registry: &AgentRegistry,
    authority: Option<&dyn BindingRecoveryAuthority>,
) -> AgentBinding {
    let policy = registry
        .adapter(&binding.kind)
        .map_or(SessionGenerationPolicy::AuthoritativeOnly, |adapter| {
            adapter.session_generation_policy()
        });
    policy.normalize_recovery_availability(&mut binding);
    let authoritative = policy.recovery_availability_is_valid(&binding)
        && authority.is_some_and(|value| value.authorizes(&binding));
    if policy == SessionGenerationPolicy::HookRequired && !authoritative {
        let unmanaged_observation = binding.state == AgentState::Unknown
            && binding.recovery_availability == AgentRecoveryAvailability::UnmanagedLaunch;
        if !unmanaged_observation {
            if binding.state != AgentState::Exited {
                binding.state = AgentState::Unknown;
            }
            binding.recovery_availability = AgentRecoveryAvailability::Unavailable;
        }
    }
    binding
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/store_tests.rs"]
mod tests;

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/store/recovery_authority_tests.rs"]
mod recovery_authority_tests;

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/store/exact_persistence_tests.rs"]
mod exact_persistence_tests;
