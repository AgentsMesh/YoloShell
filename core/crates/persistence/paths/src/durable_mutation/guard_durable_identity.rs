impl SessionDurableMutationGuard<'_> {
    pub fn session_durable_identity(
        &self,
    ) -> Result<crate::paths::identity::DurableObjectIdentity> {
        self.verify_session_directory()?;
        let identity = match self.durable_identity.get() {
            Some(identity) => *identity,
            None => self.session_directory.durable_identity()?,
        };
        self.verify_session_directory()?;
        // Store only after successful durable capture and both live fences.
        // Another successful caller may initialize the same guard concurrently.
        let cached = self.durable_identity.get_or_init(|| identity);
        ensure!(*cached == identity, SESSION_DIRECTORY_MISMATCH);
        Ok(*cached)
    }
}

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/durable/durable_mutation_directory_cache.rs"]
mod durable_identity_tests;
