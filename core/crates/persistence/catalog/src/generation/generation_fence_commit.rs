impl Catalog {
    fn write_for_controller<T>(
        &mut self,
        expected: u64,
        operation: impl FnOnce(&Transaction<'_>) -> Result<T>,
    ) -> Result<ControllerWrite<T>> {
        self.write_for_controller_with_commit_fence(expected, operation, || Ok(()))
    }

    pub fn insert_for_controller_with_commit_fence<F>(
        &mut self,
        expected: u64,
        manifest: &TerminalManifest,
        commit_fence: F,
    ) -> Result<ControllerWrite<()>>
    where
        F: FnOnce() -> Result<()>,
    {
        validate_session_manifest(manifest)?;
        self.write_for_controller_with_commit_fence(
            expected,
            |transaction| write_manifest(transaction, constants::INSERT_SESSION_SQL, manifest),
            commit_fence,
        )
    }

    pub fn upsert_for_controller_with_commit_fence<F>(
        &mut self,
        expected: u64,
        manifest: &TerminalManifest,
        commit_fence: F,
    ) -> Result<ControllerWrite<()>>
    where
        F: FnOnce() -> Result<()>,
    {
        validate_session_manifest(manifest)?;
        self.write_for_controller_with_commit_fence(
            expected,
            |transaction| write_manifest(transaction, constants::UPSERT_SESSION_SQL, manifest),
            commit_fence,
        )
    }

    fn write_for_controller_with_commit_fence<T, F>(
        &mut self,
        expected: u64,
        operation: impl FnOnce(&Transaction<'_>) -> Result<T>,
        commit_fence: F,
    ) -> Result<ControllerWrite<T>>
    where
        F: FnOnce() -> Result<()>,
    {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if generation::read_generation(&transaction)? != expected {
            return Ok(ControllerWrite::Fenced);
        }
        let result = operation(&transaction)?;
        commit_fence()?;
        transaction.commit()?;
        Ok(ControllerWrite::Applied(result))
    }
}
