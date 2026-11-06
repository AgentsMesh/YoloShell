use crate::agent::claude::generation::AuthorityRuntimeIdentity;
use crate::agent::codex::authority::{
    CodexAuthorityStatus, CodexAuthorityVersion, CodexBindingAuthority,
};
use crate::agent::hooks::{Generation, ProcessIdentity};
use crate::agent::{AgentBinding, AgentBindingStore};
use crate::namespace_lease::NamespaceLease;
use anyhow::Result;
use uuid::Uuid;

const ROUTE_REVISION: u64 = 1;
const AUTHORITY_REVISION: u64 = 1;
const EVENT_SEQUENCE: u64 = 1;

pub(super) fn publish(
    store: &AgentBindingStore,
    namespace: &NamespaceLease,
    runtime: AuthorityRuntimeIdentity,
    provider: ProcessIdentity,
    binding: &AgentBinding,
) -> Result<()> {
    let authority = if binding.state == crate::agent::AgentState::Exited {
        CodexBindingAuthority::new_with_verified_resume_identity(
            runtime,
            provider,
            crate::codex_hooks::ACCEPTED_PROVIDER_VERSION,
            CodexAuthorityVersion {
                route_revision: ROUTE_REVISION,
                invocation_id: Uuid::new_v4(),
                generation: Generation::INITIAL,
                authority_revision: AUTHORITY_REVISION,
                sequence: EVENT_SEQUENCE,
            },
            CodexAuthorityStatus::Active,
            binding.clone(),
            &crate::process::boot_id(),
        )?
    } else {
        CodexBindingAuthority::new(
            runtime,
            provider,
            crate::codex_hooks::ACCEPTED_PROVIDER_VERSION,
            CodexAuthorityVersion {
                route_revision: ROUTE_REVISION,
                invocation_id: Uuid::new_v4(),
                generation: Generation::INITIAL,
                authority_revision: AUTHORITY_REVISION,
                sequence: EVENT_SEQUENCE,
            },
            CodexAuthorityStatus::Active,
            binding.clone(),
        )?
    };
    store
        .codex_authority_store()?
        .publish_in_namespace_with_commit_fence(namespace, store.path(), &authority, || Ok(()))
}
