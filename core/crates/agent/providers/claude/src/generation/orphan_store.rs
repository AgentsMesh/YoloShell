use super::{ClaudeGenerationAuthority, ClaudeGenerationStore};
use crate::namespace_lease::NamespaceLease;
use anyhow::Result;

impl ClaudeGenerationStore {
    pub fn authorities_in_namespace(
        &self,
        namespace: &NamespaceLease,
    ) -> Result<Vec<ClaudeGenerationAuthority>> {
        Ok(self
            .load_set(namespace)?
            .map(|state| state.authorities.into_iter().collect())
            .unwrap_or_default())
    }
}
