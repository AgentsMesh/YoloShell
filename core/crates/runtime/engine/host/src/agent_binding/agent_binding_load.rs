impl Host {
    pub(crate) fn load_agent_binding(&self, manifest: &TerminalManifest) -> Result<Option<AgentBinding>> {
        self.load_agent_binding_with_authority(manifest)
            .map(|(binding, _)| binding)
    }

    /// Returns the binding and the exact runtime epoch its authority was
    /// validated against. A view can compare that epoch with the runtime
    /// observation it is about to render instead of holding the session lock
    /// across a long read RPC.
    pub(crate) fn load_agent_binding_with_authority(
        &self,
        manifest: &TerminalManifest,
    ) -> Result<(Option<AgentBinding>, Option<Uuid>)> {
        let store = self.agent_binding_store(manifest.terminal_id);
        let context = self.agent_registry.as_ref()
            .detection_context(
                manifest.terminal_id,
                manifest.cwd.clone(),
                &Default::default(),
            )
            .context(AGENT_CONTEXT_UNAVAILABLE)?;
        let runtime = self.agent_authority_runtime(manifest.terminal_id)?;
        let epoch = runtime.as_ref().map(|identity| identity.runtime_epoch);
        let binding =
            store.load_with_runtime_authority_in_namespace(&self.namespace, &context, runtime)?;
        Ok((binding, epoch))
    }

    pub(crate) fn agent_authority_runtime(
        &self,
        terminal_id: Uuid,
    ) -> Result<Option<crate::agent::authority::AuthorityRuntimeIdentity>> {
        let path = self.paths.runtime_record(terminal_id);
        if self.namespace.file_is_missing(&path)? {
            return Ok(None);
        }
        let record = self.read_namespace_json::<RuntimeRecord>(&path)?;
        anyhow::ensure!(
            record.schema_version == RUNTIME_RECORD_SCHEMA_VERSION
                && record.terminal_id == terminal_id
                && !record.runtime_epoch.is_nil()
                && record.worker_pid != 0
                && record.worker_start_time != 0,
            AGENT_CONTEXT_UNAVAILABLE
        );
        Ok(Some(
            crate::agent::authority::AuthorityRuntimeIdentity {
                runtime_epoch: record.runtime_epoch,
                worker: crate::agent::authority::AuthorityProcessIdentity {
                    pid: record.worker_pid,
                    start_time: record.worker_start_time,
                },
            },
        ))
    }
}
