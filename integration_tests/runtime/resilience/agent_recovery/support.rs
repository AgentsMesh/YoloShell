use super::*;

pub(crate) fn spawn_agent_host(home: &Path, provider: ProviderFixture, bin: &Path) -> SpawnedHost {
    prepare_host_home(home);
    install_provider_hooks(home, provider);
    let shell = home.join(TEST_SHELL);
    install_test_shell(&shell);
    let inherited_path = std::env::var_os(PATH_ENV).unwrap_or_default();
    let joined_path = std::env::join_paths(
        std::iter::once(bin.to_path_buf()).chain(std::env::split_paths(&inherited_path)),
    )
    .unwrap();
    let mut command = Command::new(runtime_binary());
    command
        .args(yolo_launch_contract::ProcessRole::Host.arguments())
        .arg(cli::SERVE_COMMAND)
        .arg(cli::HOME_ARGUMENT)
        .arg(home)
        .env_remove(provider.data_home_environment)
        .env(PATH_ENV, joined_path)
        .env(SHELL_ENV, shell)
        .stdout(Stdio::null());
    spawn_logged_host(home, &mut command)
}

fn install_provider_hooks(home: &Path, provider: ProviderFixture) {
    let spec = if provider.kind == yolo_runtime::agent::codex::constants::KIND {
        crate::support::codex_install_spec()
    } else {
        crate::support::claude_install_spec()
    };
    let home = home.join(provider.data_home_directory);
    let preview = yolo_hook_installation::inspect(spec, &home).unwrap();
    yolo_hook_installation::install(spec, &home, &preview.revision).unwrap();
}

fn install_test_shell(path: &Path) {
    std::fs::write(path, TEST_SHELL_CONTENTS).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(TEST_EXECUTABLE_MODE)).unwrap();
}

pub(crate) fn install_fake_agent(provider: ProviderFixture, path: &Path) {
    std::fs::copy(fake_agent_binary(provider), path).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(TEST_EXECUTABLE_MODE)).unwrap();
}

fn fake_agent_binary(provider: ProviderFixture) -> PathBuf {
    bazel_runfile(provider.binary_runfile)
        .expect("fake agent binary is unavailable in Bazel runfiles")
}

pub(crate) async fn wait_for_binding(client: &Client, id: Uuid, harness: &Harness) -> TerminalView {
    wait_for_view(
        client,
        id,
        |view| {
            view.agent.as_ref().is_some_and(|binding| {
                binding.recovery_availability
                    == yolo_runtime::agent::AgentRecoveryAvailability::Available
            })
        },
        Some(harness),
        harness,
    )
    .await
}

pub(crate) async fn wait_for_restored_binding(
    client: &Client,
    id: Uuid,
    expected: &yolo_runtime::agent::AgentBinding,
    harness: &Harness,
) -> TerminalView {
    wait_for_view(
        client,
        id,
        |view| {
            view.runtime
                .as_ref()
                .is_some_and(|runtime| runtime.status == TerminalRuntimeStatus::Running)
                && view.manifest.recovery_state == RecoveryState::Resumed
                && view.agent.as_ref().is_some_and(|binding| {
                    binding.kind == expected.kind
                        && binding.provider_session_id == expected.provider_session_id
                        && binding.recovery_availability
                            == yolo_runtime::agent::AgentRecoveryAvailability::Available
                        && matches!(
                            binding.state,
                            AgentState::Running | AgentState::WaitingForInput
                        )
                        && binding.process_pid.is_some()
                        && binding.process_pid != expected.process_pid
                })
        },
        Some(harness),
        harness,
    )
    .await
}

pub(crate) async fn wait_for_running_recovery(
    client: &Client,
    id: Uuid,
    state: RecoveryState,
    harness: &Harness,
) -> TerminalView {
    wait_for_view(
        client,
        id,
        |view| {
            view.manifest.recovery_state == state
                && view
                    .runtime
                    .as_ref()
                    .is_some_and(|runtime| runtime.status == TerminalRuntimeStatus::Running)
        },
        Some(harness),
        harness,
    )
    .await
}

pub(crate) async fn wait_for_recovery_state(
    client: &Client,
    id: Uuid,
    state: RecoveryState,
    harness: &Harness,
) -> TerminalView {
    wait_for_view(
        client,
        id,
        |view| view.manifest.recovery_state == state,
        None,
        harness,
    )
    .await
}

pub(crate) fn assert_stable_binding(
    previous: Option<&yolo_runtime::agent::AgentBinding>,
    restored: Option<&yolo_runtime::agent::AgentBinding>,
    provider: ProviderFixture,
    data_home: &Path,
) {
    let previous = previous.expect("previous binding");
    let restored = restored.expect("restored binding");
    assert_eq!(restored.kind, previous.kind);
    assert_eq!(restored.provider_session_id, previous.provider_session_id);
    assert_eq!(restored.evidence_path, previous.evidence_path);
    assert_eq!(restored.cwd, previous.cwd);
    assert_eq!(restored.kind.as_str(), provider.kind);
    assert_eq!(restored.executable, previous.executable);
    assert_eq!(
        restored.recovery_availability,
        yolo_runtime::agent::AgentRecoveryAvailability::Available
    );
    assert!(matches!(
        restored.state,
        AgentState::Running | AgentState::WaitingForInput
    ));
    assert_eq!(
        Path::new(&restored.executable)
            .file_name()
            .and_then(|name| name.to_str()),
        Some(provider.executable)
    );
    assert_eq!(
        Path::new(&restored.data_home).canonicalize().unwrap(),
        data_home.canonicalize().unwrap()
    );
}
