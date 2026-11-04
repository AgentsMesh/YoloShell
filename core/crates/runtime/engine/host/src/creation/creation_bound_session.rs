use super::*;

impl Host {
    #[cfg(test)]
    pub(crate) async fn prepare_bound_session(
        &self,
        binding: &CreationBinding,
    ) -> std::result::Result<TerminalManifest, ErrorBody> {
        let deadline = HostOperationDeadline::after(self.worker_ready_timeout);
        self.prepare_bound_session_before(binding, deadline).await
    }

    pub(crate) async fn prepare_bound_session_before(
        &self,
        binding: &CreationBinding,
        deadline: HostOperationDeadline,
    ) -> std::result::Result<TerminalManifest, ErrorBody> {
        if let Some(manifest) = self
            .existing_bound_manifest_before(binding, deadline)
            .await?
        {
            return Ok(manifest);
        }
        let terminal_id = binding.result.terminal_id;
        self.ensure_session_namespace_before(terminal_id, deadline)?;
        let now = now_ms();
        let shell = shell_command(&binding.spec.cwd);
        let manifest =
            TerminalManifest {
                schema_version: SESSION_MANIFEST_SCHEMA_VERSION,
                terminal_id,
                name: binding.spec.name.clone().unwrap_or_else(|| {
                    command_basename(shell.executable.to_string_lossy().as_ref())
                }),
                command: shell.executable.display().to_string(),
                args: shell.args,
                cwd: binding.spec.cwd.clone(),
                cols: normalize_terminal_columns(binding.spec.cols.unwrap_or(DEFAULT_COLS)),
                rows: normalize_terminal_rows(binding.spec.rows.unwrap_or(DEFAULT_ROWS)),
                created_at_ms: now,
                updated_at_ms: now,
                archived: false,
                recovery_state: RecoveryState::Created,
            };
        self.write_creation_json_before(&self.paths.manifest(terminal_id), &manifest, deadline)?;
        self.catalog_insert_before(&manifest, deadline)
            .await
            .map_err(creation_publication_error)?;
        Ok(manifest)
    }
}
