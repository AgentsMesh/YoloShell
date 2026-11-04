use crate::host::tests::*;
const PROVIDER_WAIT_TEST_TIMEOUT: Duration = Duration::from_millis(120);
const PROVIDER_WAIT_SUCCESS_TIMEOUT: Duration = RESUME_PROVIDER_BINDING_TIMEOUT;
const PROVIDER_WAIT_TEST_POLL_INTERVAL: Duration = Duration::from_millis(5);
const PROVIDER_WAIT_IMMEDIATE_BOUND: Duration = Duration::from_millis(500);
const PROVIDER_WAIT_LONG_TIMEOUT: Duration = Duration::from_secs(2);
const PROVIDER_WAIT_TIMEOUT_DETAIL: &str = "provider binding rejection waited for the deadline";
const PROVIDER_RUNTIME_DRIFT_TIMEOUT_DETAIL: &str =
    "provider binding wait ignored runtime record drift";
const PROVIDER_SHELL_WAIT_TIMEOUT_DETAIL: &str =
    "shell resume target entered provider binding wait";
const PROVIDER_POST_COMMIT_TIMEOUT_DETAIL: &str =
    "post-commit provider validation entered pre-commit binding wait";

struct ProviderWaitFixture {
    _agent_process: CompletionProviderProcess,
    process_scanner: Arc<ScriptedProviderProcessScanner>,
    host: Host,
    manifest: TerminalManifest,
    planned: AgentBinding,
    transaction: ResumeTransaction,
    runtime: RuntimeRecord,
    directory: tempfile::TempDir,
    authority_publisher: ProviderAuthorityPublisher,
}

impl ProviderWaitFixture {
    fn new(wait_timeout: Duration) -> Self {
        let directory = tempdir().unwrap();
        let mut host = Host::open(HostConfig::new(directory.path())).unwrap();
        host.resume_provider_binding_timeout = wait_timeout;
        host.resume_provider_binding_poll_interval = PROVIDER_WAIT_TEST_POLL_INTERVAL;
        let mut manifest = fixture_manifest(directory.path());
        manifest.recovery_state = RecoveryState::Restoring;
        host.paths.ensure_session(manifest.terminal_id).unwrap();
        let planned = completion_provider_binding(&directory, manifest.terminal_id);
        let selected = ResumeBinding::Provider(planned.clone());
        let launch = host
            .prepare_resume_launch(
                &manifest,
                &selected,
                ResumeIntent::Automatic,
                Uuid::new_v4(),
            )
            .unwrap();
        let transaction = ResumeTransaction::new(
            Uuid::new_v4(),
            ResumeIntent::Automatic,
            ResumeTarget::from_binding(&selected),
            &launch,
        )
        .unwrap();
        let agent_process = CompletionProviderProcess::spawn();
        let runtime = live_resume_runtime(&host, &transaction, BootstrapDelivery::Delivered);
        let mut resumed = planned.clone();
        resumed.process_pid = Some(agent_process.agent().pid);
        resumed.process_start_time = Some(agent_process.agent().start_time);
        resumed.updated_at_ms = resumed
            .updated_at_ms
            .saturating_add(COMPLETION_PROVIDER_TIMESTAMP_INCREMENT);
        let process = completion_provider_process(
            &resumed,
            &runtime,
            provider_arguments(
                crate::agent::codex::constants::EXECUTABLE,
                crate::agent::codex::constants::RESUME_ARGUMENT,
                COMPLETION_PROVIDER_SESSION_ID,
            ),
            Some(completion_provider_environment(&runtime)),
        );
        let process_scanner = Arc::new(ScriptedProviderProcessScanner::stable(vec![process]));
        host.provider_process_scanner = process_scanner.clone();
        persist_host_session(&host, &manifest, &runtime);
        host.publish_resume_transaction(&transaction).unwrap();
        host.publish_worker_launch(&launch).unwrap();
        Self {
            _agent_process: agent_process,
            process_scanner,
            host,
            manifest,
            planned,
            transaction,
            runtime,
            directory,
            authority_publisher: ProviderAuthorityPublisher::new(),
        }
    }

    fn resumed_binding(&self) -> AgentBinding {
        let mut binding = self.planned.clone();
        binding.process_pid = Some(self._agent_process.agent().pid);
        binding.process_start_time = Some(self._agent_process.agent().start_time);
        binding.updated_at_ms = binding
            .updated_at_ms
            .saturating_add(COMPLETION_PROVIDER_TIMESTAMP_INCREMENT);
        binding
    }

    fn valid_process(&self) -> crate::agent::ObservedProcess {
        completion_provider_process(
            &self.resumed_binding(),
            &self.runtime,
            provider_arguments(
                crate::agent::codex::constants::EXECUTABLE,
                crate::agent::codex::constants::RESUME_ARGUMENT,
                COMPLETION_PROVIDER_SESSION_ID,
            ),
            Some(completion_provider_environment(&self.runtime)),
        )
    }

    fn set_scan(&self, scan: ScriptedProviderScan) {
        self.process_scanner.set_scan(scan);
    }

    fn binding_path(&self) -> PathBuf {
        self.host.paths.agent_binding(self.transaction.terminal_id)
    }

    fn publish_binding(&self, binding: &AgentBinding) {
        self.authority_publisher
            .publish(&self.host, binding)
            .unwrap();
        let store = AgentBindingStore::new(self.binding_path());
        let context = yolo_agent_adapter_contract::DetectionContext::new(
            binding.terminal_id,
            PathBuf::from(&binding.cwd),
            [(binding.kind.clone(), PathBuf::from(&binding.data_home))],
        );
        let runtime = self
            .host
            .agent_authority_runtime(binding.terminal_id)
            .unwrap()
            .unwrap();
        store
            .repair_native_binding_in_namespace_with_commit_fence(
                &self.host.namespace,
                &context,
                runtime,
                || Ok(()),
            )
            .unwrap();
    }

    fn publish_raw_binding(&self, binding: &AgentBinding) {
        self.host
            .write_namespace_json(&self.binding_path(), binding)
            .unwrap();
    }

    fn assert_authority_files_retained(&self) {
        assert!(
            self.host
                .paths
                .resume_transaction(self.transaction.terminal_id)
                .exists()
        );
        assert!(
            self.host
                .paths
                .worker_launch(self.transaction.terminal_id)
                .exists()
        );
    }
}

async fn await_immediate_provider_rejection(
    fixture: &ProviderWaitFixture,
) -> ProviderCompletionError {
    tokio::time::timeout(
        PROVIDER_WAIT_IMMEDIATE_BOUND,
        fixture.host.await_authoritative_resume_binding(
            &fixture.manifest,
            &fixture.transaction,
            &fixture.runtime,
        ),
    )
    .await
    .expect(PROVIDER_WAIT_TIMEOUT_DETAIL)
    .unwrap_err()
}
