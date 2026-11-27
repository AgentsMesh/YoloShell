use crate::host::tests::*;
pub(super) const PROVIDER_AUTHORITY_FIRST_ROUTE_REVISION: u64 = 1;
pub(super) const PROVIDER_AUTHORITY_REVISION: u64 = 1;
pub(super) const PROVIDER_AUTHORITY_SEQUENCE: u64 = 1;
const PROVIDER_AUTHORITY_ROUTE_EXHAUSTED: &str = "provider test route revision exhausted";

pub(super) struct ProviderAuthorityPublisher {
    next_route_revision: std::sync::atomic::AtomicU64,
}

impl ProviderAuthorityPublisher {
    pub(super) fn new() -> Self {
        Self {
            next_route_revision: std::sync::atomic::AtomicU64::new(
                PROVIDER_AUTHORITY_FIRST_ROUTE_REVISION,
            ),
        }
    }

    pub(super) fn publish(&self, host: &Host, binding: &AgentBinding) -> Result<()> {
        let route_revision = self
            .next_route_revision
            .fetch_update(
                std::sync::atomic::Ordering::Relaxed,
                std::sync::atomic::Ordering::Relaxed,
                |current| current.checked_add(PROVIDER_AUTHORITY_FIRST_ROUTE_REVISION),
            )
            .map_err(|_| anyhow::anyhow!(PROVIDER_AUTHORITY_ROUTE_EXHAUSTED))?;
        let provider = crate::agent::hooks::ProcessIdentity::new(
            binding.process_pid.context(AGENT_CONTEXT_UNAVAILABLE)?,
            binding
                .process_start_time
                .context(AGENT_CONTEXT_UNAVAILABLE)?,
        );
        let runtime = host
            .agent_authority_runtime(binding.terminal_id)?
            .context(AGENT_CONTEXT_UNAVAILABLE)?;
        let authority = crate::agent::codex::authority::CodexBindingAuthority::new(
            runtime,
            provider,
            crate::codex_hooks::ACCEPTED_PROVIDER_VERSION,
            crate::agent::codex::authority::CodexAuthorityVersion {
                route_revision,
                invocation_id: Uuid::new_v4(),
                generation: crate::agent::hooks::Generation::INITIAL,
                authority_revision: PROVIDER_AUTHORITY_REVISION,
                sequence: PROVIDER_AUTHORITY_SEQUENCE,
            },
            crate::agent::codex::authority::CodexAuthorityStatus::Active,
            binding.clone(),
        )?
        .with_verified_resume_identity(&host.boot_id)?;
        let store = AgentBindingStore::new(host.paths.agent_binding(binding.terminal_id));
        store
            .codex_authority_store()?
            .publish_in_namespace_with_commit_fence(
                &host.namespace,
                store.path(),
                &authority,
                || Ok(()),
            )
    }
}
