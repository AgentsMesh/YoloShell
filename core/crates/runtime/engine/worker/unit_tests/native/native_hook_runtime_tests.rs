use crate::worker::tests::*;
#[test]
fn native_hook_runtime_fence_rejects_same_value_runtime_replacement() {
    let fixture = Fixture::new();
    let fence = claude_hook::native_runtime::RuntimeFence::capture(&fixture.state).unwrap();
    fence.verify(&fixture.state).unwrap();
    let record = lock_recover(&fixture.state.record).clone();
    atomic_write_json(
        &fixture.state.paths.runtime_record(fixture.terminal_id()),
        &record,
    )
    .unwrap();
    assert!(fence.verify(&fixture.state).is_err());
}

#[test]
fn native_hook_runtime_fence_rejects_changed_runtime_and_termination() {
    for terminate in [false, true] {
        let fixture = Fixture::new();
        let fence = claude_hook::native_runtime::RuntimeFence::capture(&fixture.state).unwrap();
        if terminate {
            fixture
                .state
                .begin_termination_for_test(ExitReason::UserTerminate);
        } else {
            lock_recover(&fixture.state.record).runtime_epoch = Uuid::new_v4();
        }
        assert!(fence.verify(&fixture.state).is_err());
    }
}

#[test]
fn native_hook_runtime_fence_rejects_successor_launch_and_namespace_replacement() {
    let fixture = Fixture::new();
    let fence = claude_hook::native_runtime::RuntimeFence::capture(&fixture.state).unwrap();
    let launch = fixture.state.paths.worker_launch(fixture.terminal_id());
    atomic_write_json(&launch, &serde_json::json!({})).unwrap();
    assert!(fence.verify(&fixture.state).is_err());
    std::fs::remove_file(&launch).unwrap();
    fence.verify(&fixture.state).unwrap();
    let record = fixture.state.paths.runtime_record(fixture.terminal_id());
    std::fs::remove_file(record).unwrap();
    assert!(fence.verify(&fixture.state).is_err());
}

#[test]
fn native_hook_context_never_changes_shell_path_or_arguments() {
    const ORIGINAL_PATH: &str = "/usr/bin:/bin:/usr/local/bin";
    const SHELL_EXECUTABLE: &str = "/bin/sh";
    let fixture = Fixture::new();
    let context = NativeHookContext::new(
        &fixture.state.namespace,
        &fixture.state.paths,
        fixture.terminal_id(),
        std::env::current_exe().unwrap().canonicalize().unwrap(),
    )
    .unwrap();
    let mut command = CommandBuilder::new(SHELL_EXECUTABLE);
    command.arg(NATIVE_HOOK_SHELL_ARGUMENT);
    command.env(crate::agent::constants::PATH_ENV, ORIGINAL_PATH);
    let arguments = command.get_argv().to_owned();
    context.configure(&mut command);
    assert_eq!(command.get_argv(), &arguments);
    assert_eq!(
        command.get_env(crate::agent::constants::PATH_ENV),
        Some(std::ffi::OsStr::new(ORIGINAL_PATH))
    );
    assert_eq!(
        command.get_env(crate::codex_hooks::NATIVE_HOOK_EXECUTABLE_ENV),
        Some(context.executable.as_os_str())
    );
    assert_eq!(
        command.get_env(crate::codex_hooks::NATIVE_HOOK_HOME_ENV),
        Some(fixture.state.paths.home().as_os_str())
    );
    assert_eq!(
        command.get_env(crate::codex_hooks::SOCKET_ENV),
        Some(context.codex_socket.as_os_str())
    );
}
