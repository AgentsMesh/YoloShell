use crate::agent::launch::apply_resume_with_registry;
#[derive(Clone, Copy)]
pub(super) enum UnstartedLaunchPolicy {
    Retire,
    Preserve,
}

impl Host {
    pub(super) async fn spawn_creation_worker(
        &self,
        manifest: &TerminalManifest,
        binding: &CreationBinding,
        planned: &CreationLaunchIdentity,
        deadline: HostOperationDeadline,
    ) -> Result<WorkerStart> {
        let launch = self.prepare_creation_worker_launch(binding, planned)?;
        self.publish_creation_worker_launch_before(&launch, deadline)?;
        self.spawn_published_worker_before(
            manifest,
            &launch,
            UnstartedLaunchPolicy::Retire,
            deadline,
        )
        .await
    }

    pub(super) fn prepare_worker_launch(
        &self,
        manifest: &TerminalManifest,
        command: ShellCommand,
        resume: Option<(&crate::agent::AgentBinding, ResumeIntent)>,
        planned: Option<&CreationLaunchIdentity>,
    ) -> Result<WorkerLaunchSpec> {
        let runtime_epoch = planned
            .map(|identity| identity.runtime_epoch)
            .unwrap_or_else(Uuid::new_v4);
        let worker_id = planned
            .map(|identity| identity.worker_id)
            .unwrap_or_else(Uuid::new_v4);
        let auth_token = planned
            .map(|identity| identity.auth_token.clone())
            .unwrap_or_else(|| format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple()));
        let mut launch = WorkerLaunchSpec {
            schema_version: WORKER_LAUNCH_SCHEMA_VERSION,
            terminal_id: manifest.terminal_id,
            runtime_epoch,
            worker_id,
            socket_path: self.paths.worker_socket(manifest.terminal_id)?,
            auth_token,
            controller_generation: self.controller_generation,
            command: command.executable.to_string_lossy().into_owned(),
            args: command.args,
            cwd: command.cwd,
            environment: Default::default(),
            initial_input: None,
            cols: normalize_terminal_columns(manifest.cols),
            rows: normalize_terminal_rows(manifest.rows),
            context: None,
        };
        if let Some((binding, intent)) = resume {
            apply_resume_with_registry(
                &mut launch,
                binding,
                intent,
                self.agent_registry.as_ref(),
            )?;
        }
        Ok(launch)
    }

    pub(super) fn publish_worker_launch(&self, launch: &WorkerLaunchSpec) -> Result<()> {
        self.write_namespace_json(&self.paths.worker_launch(launch.terminal_id), launch)
    }

    fn spawn_session_worker_before(
        &self,
        arguments: &[std::ffi::OsString],
        deadline: HostOperationDeadline,
    ) -> Result<process::command_worker::WorkerProcess> {
        let mut command = std::process::Command::new(&self.runtime_executable);
        command.args(arguments);
        spawn_host_worker_command_before(
            &self.instance_ownership,
            command,
            deadline.process_instant(),
        )
    }

    pub(super) async fn spawn_published_worker(
        &self,
        manifest: &TerminalManifest,
        launch: &WorkerLaunchSpec,
        failure_policy: UnstartedLaunchPolicy,
    ) -> Result<WorkerStart> {
        let deadline = HostOperationDeadline::after(self.worker_ready_timeout);
        self.spawn_published_worker_before(manifest, launch, failure_policy, deadline)
            .await
    }

    pub(super) fn apply_unstarted_launch_policy(
        &self,
        terminal_id: Uuid,
        policy: UnstartedLaunchPolicy,
    ) -> Result<()> {
        match policy {
            UnstartedLaunchPolicy::Retire => self.retire_unstarted_launch(terminal_id),
            UnstartedLaunchPolicy::Preserve => Ok(()),
        }
    }

    pub(super) async fn await_worker_start(
        &self,
        manifest: &TerminalManifest,
        runtime_epoch: Uuid,
        worker_id: Uuid,
    ) -> Result<WorkerStart> {
        let deadline = HostOperationDeadline::after(self.worker_ready_timeout);
        self.await_worker_start_before(manifest, runtime_epoch, worker_id, deadline)
            .await
    }

    pub(super) fn classify_worker_start_timeout(
        &self,
        manifest: &TerminalManifest,
        runtime_epoch: Uuid,
        worker_id: Uuid,
    ) -> Result<WorkerStart> {
        let record = self
            .read_namespace_json::<RuntimeRecord>(&self.paths.runtime_record(manifest.terminal_id));
        let pending = record.ok().filter(|record| {
            record.runtime_epoch == runtime_epoch
                && record.worker_id == worker_id
                && record.bootstrap_delivery == BootstrapDelivery::Pending
        });
        let detail = match pending.as_ref() {
            Some(record) if pending_bootstrap_identities_live(record) => {
                "worker bootstrap remained pending before timeout"
            }
            Some(_) => "worker bootstrap remained pending after a process identity exited",
            None => "worker did not become ready before timeout",
        };
        Ok(WorkerStart::Uncertain(anyhow::anyhow!(detail)))
    }

    fn retire_unstarted_launch(&self, terminal_id: Uuid) -> Result<()> {
        self.namespace
            .remove_file(&self.paths.worker_launch(terminal_id))
            .context(UNSTARTED_WORKER_LAUNCH_RETIRE_CONTEXT)
    }
}

pub(super) fn spawn_host_worker_command_before(
    ownership: &HostInstanceOwnership,
    command: std::process::Command,
    deadline: std::time::Instant,
) -> Result<crate::process::command_worker::WorkerProcess> {
    process::command_private_input::require_budget(deadline)?;
    let lock_descriptors = ownership.worker_lock_descriptors_for_spawn()?;
    #[cfg(all(test, feature = "yolo-integration-tests", target_os = "macos"))]
    native_worker_spawn_test_support::run(lock_descriptors)?;
    #[cfg(target_os = "macos")]
    {
        let executable = command.get_program().to_owned();
        let args = command
            .get_args()
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        let environment = command
            .get_envs()
            .map(|(key, value)| (key.to_owned(), value.map(ToOwned::to_owned)))
            .collect::<Vec<_>>();
        return process::command_worker::spawn_worker_before_deadline(
            std::path::Path::new(&executable),
            &args,
            &environment,
            &lock_descriptors,
            deadline,
        );
    }
    #[cfg(not(target_os = "macos"))]
    {
        let mut command = command;
        ownership.configure_worker_command(&mut command)?;
        process::command_worker::spawn_worker_before_deadline(
            command,
            &lock_descriptors,
            process::DetachedStandardError::Inherit,
            deadline,
        )
    }
}

#[cfg(all(test, feature = "yolo-integration-tests", target_os = "macos"))]
#[path = "../../test_support/integration/native_worker_spawn.rs"]
mod native_worker_spawn_test_support;

include!("session_spawn_deadline.rs");
include!("session_spawn_checkpoint.rs");
include!("session_spawn_creation.rs");
