use crate::host::tests::*;
pub(super) const CHECKPOINT_TEST_PROCESS_PID: u32 = i32::MAX as u32 - 20;
pub(super) const CHECKPOINT_TEST_CHILD_PID: u32 = CHECKPOINT_TEST_PROCESS_PID - 1;
pub(super) const CHECKPOINT_TEST_START_TIME: u64 = 1;
pub(super) const CHECKPOINT_TEST_IDENTITY_OFFSET: u64 = 1;
const CHECKPOINT_TEST_MALFORMED_BYTES: &[u8] = b"{invalid-checkpoint";
const CHECKPOINT_TEST_SESSION_NAME: &str = "startup-checkpoint";
const CHECKPOINT_TEST_MISSING_RUNTIME: &str = "missing-checkpoint-runtime";

pub(super) struct CreationCheckpointFixture {
    pub(super) host: Host,
    pub(super) request: CreateTerminal,
    pub(super) ledger_path: PathBuf,
    pub(super) binding: CreationBinding,
    pub(super) planned: CreationLaunchIdentity,
    pub(super) launch: WorkerLaunchSpec,
    pub(super) _directory: HostTestDirectory,
}

impl CreationCheckpointFixture {
    pub(super) async fn new(publish_launch: bool) -> Self {
        let directory = tempdir().unwrap();
        let host = Host::open(HostConfig::new(directory.path())).unwrap();
        let (request, reserved, ledger_path) =
            reserve_test_creation(&host, directory.path(), CHECKPOINT_TEST_SESSION_NAME);
        host.prepare_bound_session(&reserved).await.unwrap();
        let planned = lock_recover(&host.creation_capabilities)
            .prepare_launch(&ledger_path, reserved.result)
            .unwrap();
        let binding = lock_recover(&host.creation_capabilities)
            .bindings
            .iter()
            .find(|value| value.result == reserved.result)
            .unwrap()
            .clone();
        let launch = planned_worker_launch(&host, &binding, &planned);
        if publish_launch {
            atomic_write_json(
                &host.paths.worker_launch(binding.result.terminal_id),
                &launch,
            )
            .unwrap();
        }
        Self {
            _directory: HostTestDirectory::capture(directory, &host.paths),
            host,
            request,
            ledger_path,
            binding,
            planned,
            launch,
        }
    }

    pub(super) fn checkpoint(
        &self,
        stage: crate::worker::StartupStage,
        phase: crate::worker::StartupPhase,
        status: crate::worker::StartupStatus,
        child: Option<crate::worker::StartupProcessIdentity>,
    ) -> crate::worker::StartupCheckpointEvidence {
        let launch_fence =
            crate::worker::worker_startup_launch_fence_for_test(&self.launch).unwrap();
        crate::worker::StartupCheckpointEvidence {
            schema_version: crate::worker::TEST_WORKER_STARTUP_CHECKPOINT_SCHEMA_VERSION,
            terminal_id: self.binding.result.terminal_id,
            runtime_epoch: self.planned.runtime_epoch,
            worker_id: self.planned.worker_id,
            controller_generation: self.host.controller_generation,
            launch_sha256: launch_fence.launch_sha256,
            boot_id: crate::process::boot_id(),
            worker: checkpoint_process(CHECKPOINT_TEST_PROCESS_PID),
            child,
            stage,
            phase,
            status,
            successor: None,
            predecessor_runtime_sha256: None,
            pending_pty_custody_rollover: None,
        }
    }

    pub(super) fn active_checkpoint(&self) -> crate::worker::StartupCheckpointEvidence {
        self.checkpoint(
            crate::worker::StartupStage::LaunchContract,
            crate::worker::StartupPhase::Completed,
            crate::worker::StartupStatus::Active,
            None,
        )
    }

    pub(super) fn write(&self, checkpoint: &crate::worker::StartupCheckpointEvidence) {
        atomic_write_json(
            &self
                .host
                .paths
                .worker_startup_checkpoint(self.binding.result.terminal_id),
            checkpoint,
        )
        .unwrap();
    }

    pub(super) fn path(&self) -> PathBuf {
        self.host
            .paths
            .worker_startup_checkpoint(self.binding.result.terminal_id)
    }

    pub(super) fn read(&self) -> crate::worker::StartupCheckpointEvidence {
        read_json(
            &self
                .host
                .paths
                .worker_startup_checkpoint(self.binding.result.terminal_id),
        )
        .unwrap()
    }

    fn reconcile(&self) -> std::result::Result<bool, ErrorBody> {
        self.host
            .reconcile_pre_runtime_checkpoint(&self.binding, &self.planned)
    }

    pub(super) fn predecessor(&self, delivery: BootstrapDelivery) -> RuntimeRecord {
        let mut runtime = runtime_for_creation(&self.host, &self.launch, delivery, false);
        runtime.worker_pid = CHECKPOINT_TEST_PROCESS_PID;
        runtime.worker_start_time = CHECKPOINT_TEST_START_TIME;
        runtime.child_pid = CHECKPOINT_TEST_CHILD_PID;
        runtime.child_start_time = CHECKPOINT_TEST_START_TIME;
        runtime
    }

    pub(super) fn write_runtime(&self, runtime: &RuntimeRecord) {
        atomic_write_json(
            &self
                .host
                .paths
                .runtime_record(self.binding.result.terminal_id),
            runtime,
        )
        .unwrap();
    }

    pub(super) fn publish_successor(&self) -> WorkerLaunchSpec {
        let manifest: TerminalManifest =
            read_json(&self.host.paths.manifest(self.binding.result.terminal_id)).unwrap();
        let successor = shell_resume_launch(&self.host, &manifest);
        self.host.publish_worker_launch(&successor).unwrap();
        successor
    }

    pub(super) fn manifest(&self) -> TerminalManifest {
        read_json(&self.host.paths.manifest(self.binding.result.terminal_id)).unwrap()
    }

    pub(super) fn missing_runtime(&self) -> PathBuf {
        self._directory.path().join(CHECKPOINT_TEST_MISSING_RUNTIME)
    }
}

pub(super) fn checkpoint_process(pid: u32) -> crate::worker::StartupProcessIdentity {
    crate::worker::StartupProcessIdentity {
        pid,
        start_time: CHECKPOINT_TEST_START_TIME,
    }
}

pub(super) fn gone_checkpoint_processes() -> (
    crate::process::ProcessIdentityRelationGuard,
    crate::process::ProcessIdentityRelationGuard,
) {
    let worker = crate::process::install_test_process_identity_relation(
        CHECKPOINT_TEST_PROCESS_PID,
        CHECKPOINT_TEST_START_TIME,
        crate::process::ProcessIdentityRelation::Gone,
    );
    let child = crate::process::install_test_process_identity_relation(
        CHECKPOINT_TEST_CHILD_PID,
        CHECKPOINT_TEST_START_TIME,
        crate::process::ProcessIdentityRelation::Gone,
    );
    (worker, child)
}

pub(super) fn checkpoint_successor_fence(launch: &WorkerLaunchSpec) -> crate::worker::StartupLaunchFence {
    crate::worker::worker_startup_launch_fence_for_test(launch).unwrap()
}
