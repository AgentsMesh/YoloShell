impl Host {
    pub(crate) fn load_agent_binding(&self, manifest: &TerminalManifest) -> Result<Option<AgentBinding>> {
        self.load_agent_binding_with_observed_runtime(manifest, None)
            .map(|(binding, _)| binding)
    }

    pub(crate) fn load_agent_binding_with_observed_runtime(
        &self,
        manifest: &TerminalManifest,
        observed_runtime: Option<&RuntimeRecord>,
    ) -> Result<(Option<AgentBinding>, Option<Uuid>)> {
        let store = self.agent_binding_store(manifest.terminal_id);
        let context = self.agent_registry.as_ref()
            .detection_context(
                manifest.terminal_id,
                manifest.cwd.clone(),
                &Default::default(),
            )
            .context(AGENT_CONTEXT_UNAVAILABLE)?;
        let runtime = match observed_runtime {
            Some(record) => Some(
                self.agent_authority_runtime_from_record(manifest.terminal_id, record)?,
            ),
            None => self.agent_authority_runtime(manifest.terminal_id)?,
        };
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
        self.agent_authority_runtime_from_record(terminal_id, &record)
            .map(Some)
    }

    fn agent_authority_runtime_from_record(
        &self,
        terminal_id: Uuid,
        record: &RuntimeRecord,
    ) -> Result<crate::agent::authority::AuthorityRuntimeIdentity> {
        anyhow::ensure!(
            record.schema_version == RUNTIME_RECORD_SCHEMA_VERSION
                && record.terminal_id == terminal_id
                && !record.runtime_epoch.is_nil()
                && record.worker_pid != 0
                && record.worker_start_time != 0,
            AGENT_CONTEXT_UNAVAILABLE
        );
        Ok(crate::agent::authority::AuthorityRuntimeIdentity {
            runtime_epoch: record.runtime_epoch,
            worker: crate::agent::authority::AuthorityProcessIdentity {
                pid: record.worker_pid,
                start_time: record.worker_start_time,
            },
        })
    }
}
