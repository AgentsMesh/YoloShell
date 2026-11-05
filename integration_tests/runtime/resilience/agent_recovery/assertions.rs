use super::*;

pub(super) async fn assert_automatic_recovery(
    client: &Client,
    fixture: &RecoveryFixture,
    provider: ProviderFixture,
    data_home: &Path,
    runtime_snapshot: &DurableRuntimeSnapshot,
    harness: &Harness,
) {
    let terminal_id = fixture.previous.manifest.terminal_id;
    match fixture.expectation {
        AutomaticExpectation::Provider => {
            let previous_binding = fixture
                .previous
                .agent
                .as_ref()
                .expect("previous agent binding");
            let restored =
                wait_for_restored_binding(client, terminal_id, previous_binding, harness).await;
            assert_stable_binding(
                fixture.previous.agent.as_ref(),
                restored.agent.as_ref(),
                provider,
                data_home,
            );
            assert_foreground_agent_descendant(
                &restored,
                provider,
                Path::new(&previous_binding.cwd),
            );
            assert_recovered_identities_rotated(&fixture.previous, &restored);
        }
        AutomaticExpectation::ExplicitOnly => {
            let restored =
                wait_for_recovery_state(client, terminal_id, RecoveryState::NeedsRecreate, harness)
                    .await;
            let binding = restored.agent.as_ref().expect("historical agent binding");
            assert_eq!(binding.state, AgentState::Exited);
            assert_eq!(binding.process_pid, None);
            assert_eq!(
                binding.recovery_availability,
                yolo_runtime::agent::AgentRecoveryAvailability::Available
            );
            runtime_snapshot.assert_unchanged(&harness.paths());
            assert_runtime_identities_gone(&runtime_snapshot.record);
        }
        AutomaticExpectation::FailClosed => {
            let restored =
                wait_for_recovery_state(client, terminal_id, RecoveryState::NeedsRecreate, harness)
                    .await;
            let binding = restored.agent.as_ref().expect("unavailable agent binding");
            assert_eq!(binding.state, AgentState::Unknown);
            assert_eq!(
                binding.recovery_availability,
                yolo_runtime::agent::AgentRecoveryAvailability::Unavailable
            );
            runtime_snapshot.assert_unchanged(&harness.paths());
            assert_runtime_identities_gone(&runtime_snapshot.record);
        }
    }
}

pub(super) fn assert_agent_identity_gone(previous: &TerminalView) {
    let binding = previous.agent.as_ref().expect("previous agent binding");
    let identity = binding_identity(binding);
    assert!(!identity.matches());
}

pub(super) fn assert_recovered_identities_rotated(previous: &TerminalView, restored: &TerminalView) {
    assert_runtime_views_rotated(previous, restored);
    let old_agent = binding_identity(previous.agent.as_ref().expect("previous agent binding"));
    let new_agent = binding_identity(restored.agent.as_ref().expect("restored agent binding"));
    assert_process_identity_rotated(old_agent, new_agent);
}

pub(super) fn assert_runtime_views_rotated(previous: &TerminalView, restored: &TerminalView) {
    let old = previous.runtime.as_ref().expect("previous runtime");
    let new = restored.runtime.as_ref().expect("restored runtime");
    assert_ne!(new.runtime_epoch, old.runtime_epoch);
    assert_ne!(new.worker_id, old.worker_id);
    assert_process_identity_rotated(
        ProcessIdentity {
            pid: old.worker_pid,
            start_time: old.worker_start_time,
        },
        ProcessIdentity {
            pid: new.worker_pid,
            start_time: new.worker_start_time,
        },
    );
    assert_process_identity_rotated(
        ProcessIdentity {
            pid: old.child_pid,
            start_time: old.child_start_time,
        },
        ProcessIdentity {
            pid: new.child_pid,
            start_time: new.child_start_time,
        },
    );
}

fn binding_identity(binding: &yolo_runtime::agent::AgentBinding) -> ProcessIdentity {
    ProcessIdentity {
        pid: binding.process_pid.expect("agent process id"),
        start_time: binding
            .process_start_time
            .expect("agent process start time"),
    }
}

pub(super) fn assert_real_process_detection(
    view: &TerminalView,
    provider: ProviderFixture,
    expected_cwd: &Path,
    data_home: &Path,
) {
    let binding = view.agent.as_ref().expect("detected agent binding");
    assert_eq!(
        binding.recovery_availability,
        yolo_runtime::agent::AgentRecoveryAvailability::Available
    );
    let process = assert_foreground_agent_descendant(view, provider, expected_cwd);
    assert_process_arguments(&process, provider, &binding.provider_session_id);
    let expected_evidence = provider.evidence_path(data_home, &binding.provider_session_id);
    assert_same_directory(Path::new(&binding.evidence_path), &expected_evidence);
}

fn assert_foreground_agent_descendant(
    view: &TerminalView,
    provider: ProviderFixture,
    expected_cwd: &Path,
) -> yolo_runtime::agent::ObservedProcess {
    let runtime = view.runtime.as_ref().expect("detected shell runtime");
    let binding = view.agent.as_ref().expect("detected agent binding");
    let pid = binding.process_pid.expect("detected agent process id");
    let processes = RuntimeProcessSource::new([]).descendants(runtime.child_pid).unwrap();
    let process = processes
        .into_iter()
        .find(|process| process.pid == pid)
        .expect("bound agent is a real shell descendant");
    assert!(process.is_foreground());
    assert_eq!(
        Path::new(&process.executable)
            .file_name()
            .and_then(|name| name.to_str()),
        Some(provider.process_executable())
    );
    assert_same_directory(process.cwd.as_deref().unwrap(), expected_cwd);
    assert_same_directory(Path::new(&binding.cwd), expected_cwd);
    process
}

fn assert_process_arguments(
    process: &yolo_runtime::agent::ObservedProcess,
    provider: ProviderFixture,
    _provider_session_id: &str,
) {
    if provider.kind == yolo_runtime::agent::codex::constants::KIND {
        assert_managed_codex_arguments(process);
        return;
    }
    assert!(matches!(
        process.argv.as_slice(),
        [executable]
            if Path::new(executable).file_name().and_then(|name| name.to_str())
                    == Some(provider.executable)
    ));
}

fn assert_same_directory(actual: &Path, expected: &Path) {
    assert_eq!(
        actual.canonicalize().unwrap(),
        expected.canonicalize().unwrap()
    );
}
