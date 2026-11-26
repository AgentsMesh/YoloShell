use super::super::ClaudeGenerationStore;
use crate::agent::claude::generation_test_hook::{
    BindingPublicationSnapshot, GenerationPublicationSnapshot,
};
use anyhow::{Context, Result};
use std::path::Path;

const NAMESPACE_LABEL: &str = "Claude publication snapshot test";

pub(crate) fn publication_snapshot(binding_path: &Path) -> Result<GenerationPublicationSnapshot> {
    let session = binding_path
        .parent()
        .context(super::super::constants::BINDING_PARENT_MISSING)?;
    let lease = crate::namespace_lease::NamespaceLease::capture(&[(NAMESPACE_LABEL, session)])?;
    let store = ClaudeGenerationStore::from_binding_path(binding_path)?;
    let set = store
        .load_set(&lease)?
        .context(super::super::constants::AUTHORITY_MISSING)?;
    let active = set.active()?;
    let binding_publication =
        set.binding_publication
            .map(|publication| BindingPublicationSnapshot {
                invocation_id: publication.invocation_id,
                publication_generation: publication.publication_generation,
                authority_revision: publication.authority_revision,
            });
    Ok(GenerationPublicationSnapshot {
        active_invocation_id: set.active_invocation_id,
        active_revision: active.revision,
        active_binding_revision: active.binding_revision,
        publication_generation: set.publication_generation,
        binding_publication,
        has_pending_binding_transition: set.pending_binding_transition.is_some(),
    })
}
