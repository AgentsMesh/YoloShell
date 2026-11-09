const MISSING_PROCESS_START_TIME: u64 = 1;
const CREATION_TEST_COLUMNS: u16 = 91;
const CREATION_TEST_ROWS: u16 = 26;
const HOST_BINDING_AGENT_SESSION: &str = "codex-session";
const HOST_BINDING_EVIDENCE_PATH: &str = "/tmp/session.jsonl";
const HOST_BINDING_DATA_HOME: &str = "/tmp/.codex";

fn reserve_test_creation(
    host: &Host,
    directory: &Path,
    name: &str,
) -> (CreateTerminal, CreationBinding, PathBuf) {
    let request = CreateTerminal {
        creation_token: Uuid::new_v4(),
        name: Some(name.into()),
        cwd: directory.to_path_buf(),
        cols: Some(CREATION_TEST_COLUMNS),
        rows: Some(CREATION_TEST_ROWS),
        initial_input: None,
        context: None,
    };
    let fingerprint = CreationSpec::fingerprint(&request).unwrap();
    let spec = CreationSpec::resolve(&request, fingerprint).unwrap();
    let ledger_path = host.paths.home().join(CREATION_LEDGER_FILE_NAME);
    let binding = lock_recover(&host.creation_capabilities)
        .reserve(
            &ledger_path,
            request.creation_token,
            spec,
            &host.creation_receipts,
        )
        .unwrap();
    (request, binding, ledger_path)
}

fn planned_worker_launch(
    host: &Host,
    binding: &CreationBinding,
    launch: &CreationLaunchIdentity,
) -> WorkerLaunchSpec {
    let shell = shell_command(&binding.spec.cwd);
    WorkerLaunchSpec {
        schema_version: WORKER_LAUNCH_SCHEMA_VERSION,
        terminal_id: binding.result.terminal_id,
        runtime_epoch: launch.runtime_epoch,
        worker_id: launch.worker_id,
        socket_path: host
            .paths
            .worker_socket(binding.result.terminal_id)
            .unwrap(),
        auth_token: launch.auth_token.clone(),
        controller_generation: host.controller_generation,
        command: shell.executable.display().to_string(),
        args: shell.args,
        cwd: binding.spec.cwd.clone(),
        environment: Default::default(),
        initial_input: None,
        cols: binding.spec.cols.unwrap(),
        rows: binding.spec.rows.unwrap(),
        context: None,
    }
}

fn assert_transition_ledger(path: &Path, expected: &[CreationBinding]) {
    assert_eq!(CreationLedger::open(path).unwrap().bindings, expected);
}

fn host_binding(terminal_id: Uuid) -> crate::agent::AgentBinding {
    crate::agent::AgentBinding {
        schema_version: crate::agent::constants::BINDING_SCHEMA_VERSION,
        terminal_id,
        kind: crate::agent::AgentKind::Codex,
        provider_session_id: HOST_BINDING_AGENT_SESSION.into(),
        executable: crate::agent::codex::constants::EXECUTABLE.into(),
        cwd: "/tmp".into(),
        evidence_path: HOST_BINDING_EVIDENCE_PATH.into(),
        data_home: HOST_BINDING_DATA_HOME.into(),
        process_pid: None,
        process_start_time: None,
        state: crate::agent::AgentState::Exited,
        recovery_availability: crate::agent::AgentRecoveryAvailability::Available,
        detected_at_ms: 1,
        updated_at_ms: 2,
    }
}
