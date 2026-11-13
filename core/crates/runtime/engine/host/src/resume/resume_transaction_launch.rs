use crate::agent::launch::apply_resume_with_registry;
use super::*;

impl Host {
    pub(crate) fn prepare_resume_launch(
        &self,
        manifest: &TerminalManifest,
        binding: &ResumeBinding,
        intent: ResumeIntent,
        previous_runtime_epoch: Uuid,
    ) -> Result<WorkerLaunchSpec> {
        anyhow::ensure!(
            self.namespace
                .file_is_missing(&self.paths.worker_launch(manifest.terminal_id))?,
            RESUME_LAUNCH_WITHOUT_TRANSACTION
        );
        let command = binding.launch_command(manifest);
        let resume = match binding {
            ResumeBinding::Provider(binding) => Some((binding, intent)),
            ResumeBinding::Shell(_) => None,
        };
        let launch = self.prepare_worker_launch(manifest, command, resume, None)?;
        self.validate_resume_launch(manifest, &launch, binding, intent, previous_runtime_epoch)?;
        Ok(launch)
    }

    pub(crate) fn validate_resume_launch(
        &self,
        manifest: &TerminalManifest,
        launch: &WorkerLaunchSpec,
        binding: &ResumeBinding,
        intent: ResumeIntent,
        previous_runtime_epoch: Uuid,
    ) -> Result<()> {
        anyhow::ensure!(
            launch.schema_version == WORKER_LAUNCH_SCHEMA_VERSION
                && launch.terminal_id == manifest.terminal_id
                && !launch.runtime_epoch.is_nil()
                && launch.runtime_epoch != previous_runtime_epoch
                && !launch.worker_id.is_nil()
                && launch.worker_id != launch.runtime_epoch
                && launch.controller_generation != 0
                && valid_lower_hex(&launch.auth_token, WORKER_AUTH_TOKEN_HEX_LENGTH),
            RESUME_EVIDENCE_MISMATCH
        );
        let command = binding.launch_command(manifest);
        let mut expected = WorkerLaunchSpec {
            schema_version: WORKER_LAUNCH_SCHEMA_VERSION,
            terminal_id: manifest.terminal_id,
            runtime_epoch: launch.runtime_epoch,
            worker_id: launch.worker_id,
            socket_path: self.paths.worker_socket(manifest.terminal_id)?,
            auth_token: launch.auth_token.clone(),
            controller_generation: launch.controller_generation,
            command: command.executable.to_string_lossy().into_owned(),
            args: command.args,
            cwd: command.cwd,
            environment: Default::default(),
            initial_input: None,
            cols: normalize_terminal_columns(manifest.cols),
            rows: normalize_terminal_rows(manifest.rows),
            context: None,
        };
        if let ResumeBinding::Provider(binding) = binding {
            apply_resume_with_registry(
                &mut expected,
                binding,
                intent,
                self.agent_registry.as_ref(),
            )?;
        }
        // Caller context is durable launch evidence; the resume plan has no
        // separate source for it, so the persisted launch remains authoritative.
        expected.context = launch.context;
        anyhow::ensure!(&expected == launch, RESUME_EVIDENCE_MISMATCH);
        Ok(())
    }

    pub(crate) fn validate_transaction_launch(
        &self,
        manifest: &TerminalManifest,
        transaction: &ResumeTransaction,
        launch: &WorkerLaunchSpec,
    ) -> Result<()> {
        transaction.validate_launch(launch)?;
        let command = transaction.target.launch_command(manifest);
        anyhow::ensure!(
            launch.schema_version == WORKER_LAUNCH_SCHEMA_VERSION
                && launch.socket_path == self.paths.worker_socket(manifest.terminal_id)?
                && launch.command == command.executable.to_string_lossy()
                && launch.args == command.args
                && launch.cwd == command.cwd
                && normalize_terminal_columns(launch.cols)
                    == normalize_terminal_columns(manifest.cols)
                && normalize_terminal_rows(launch.rows) == normalize_terminal_rows(manifest.rows)
                && launch.controller_generation != 0
                && valid_lower_hex(&launch.auth_token, WORKER_AUTH_TOKEN_HEX_LENGTH),
            RESUME_EVIDENCE_MISMATCH
        );
        let target_matches = match &transaction.target {
            ResumeTarget::Shell { .. } => {
                launch.environment.is_empty() && launch.initial_input.is_none()
            }
            ResumeTarget::Provider { binding, .. } => {
                let mut expected = launch.clone();
                expected.environment.clear();
                expected.initial_input = None;
                apply_resume_with_registry(
                    &mut expected,
                    binding,
                    transaction.intent,
                    self.agent_registry.as_ref(),
                )?;
                expected == *launch
            }
        };
        anyhow::ensure!(target_matches, RESUME_EVIDENCE_MISMATCH);
        Ok(())
    }
}
