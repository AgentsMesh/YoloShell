impl Host {
    pub(crate) fn read_runtime_with_identity(
        &self,
        terminal_id: uuid::Uuid,
    ) -> Result<(RuntimeRecord, FileIdentity)> {
        self.namespace
            .read_json_with_identity(&self.paths.runtime_record(terminal_id))
    }

    pub(crate) fn ensure_runtime_unchanged(
        &self,
        terminal_id: uuid::Uuid,
        expected: &RuntimeRecord,
        expected_identity: FileIdentity,
    ) -> Result<()> {
        let (current, identity) = self.read_runtime_with_identity(terminal_id)?;
        anyhow::ensure!(
            current == *expected && identity == expected_identity,
            AUTOMATIC_CRASH_EXIT_RUNTIME_CHANGED
        );
        Ok(())
    }

    pub(crate) async fn ensure_socket_closed(&self, path: &Path) -> Result<()> {
        anyhow::ensure!(
            !socket_may_be_serving(path).await,
            AUTOMATIC_CRASH_EXIT_SOCKET_ACTIVE
        );
        Ok(())
    }

    pub(crate) fn read_exit_evidence(
        &self,
        path: &Path,
        runtime_epoch: uuid::Uuid,
    ) -> Result<ExitEvidence> {
        if self.namespace.file_is_missing(path)? {
            return Ok(ExitEvidence::Missing);
        }
        let (exit, identity) = self.namespace.read_json_with_identity::<ExitRecord>(path)?;
        if exit.runtime_epoch != runtime_epoch {
            return Ok(ExitEvidence::Stale(identity));
        }
        if exit.reason == ExitReason::WorkerCrash {
            return Ok(ExitEvidence::MatchingWorkerCrash { exit, identity });
        }
        anyhow::bail!(AUTOMATIC_CRASH_EXIT_NON_CRASH)
    }
}
