use super::*;
use crate::agent::claude::generation::{
    AuthorityRuntimeIdentity, ClaudeGenerationAuthority, ClaudeGenerationStore,
};

pub(super) fn load(
    store: &AgentBindingStore,
    namespace: &crate::namespace_lease::NamespaceLease,
    context: &DetectionContext,
    runtime: Option<AuthorityRuntimeIdentity>,
    authority: Option<&ClaudeGenerationAuthority>,
    binding: Option<&AgentBinding>,
) -> Result<Option<AgentBinding>> {
    let (Some(runtime), Some(authority), Some(binding)) = (runtime, authority, binding) else {
        return Ok(None);
    };
    if authority.matches_runtime(runtime)
        || authority.state != AgentState::Exited
        || binding.state != AgentState::Exited
        || !authority.matches_binding(binding)
    {
        return Ok(None);
    }
    let generations = ClaudeGenerationStore::from_binding_path(store.path())?;
    if !generations.binding_is_published_in_namespace(
        namespace,
        authority.authentication(),
        authority,
    )? {
        return Ok(None);
    }
    store.registry().validate_binding(binding, context)?;
    Ok(Some(binding.clone()))
}
