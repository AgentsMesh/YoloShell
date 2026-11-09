use super::*;
use yolo_runtime::host::reconcile_test_hook::ReconcileTestBarrier;

const RECONCILE_PANIC_SESSION_NAME: &str = "reconcile-panic-isolation";
const INVALID_RECONCILE_BARRIER_ROOT: &str = "relative-reconcile-barrier";
const RECONCILE_PANIC_CODEX_HOME: &str = "reconcile-panic-codex-home";
const EXPECTED_ISOLATED_PANICS: usize = 1;

fn spawn_reconcile_panic_host(home: &Path) -> SpawnedHost {
    prepare_host_home(home);
    let codex_home = home.join(RECONCILE_PANIC_CODEX_HOME);
    std::fs::create_dir_all(&codex_home).unwrap();
    let mut command = Command::new(runtime_binary());
    command
        .args(yolo_launch_contract::ProcessRole::Host.arguments())
        .args([cli::SERVE_COMMAND, cli::HOME_ARGUMENT])
        .arg(home)
        .env(CODEX_HOME_ENV, codex_home)
        .env(
            ReconcileTestBarrier::environment_variable(),
            INVALID_RECONCILE_BARRIER_ROOT,
        )
        .env(SHELL_ENV, TEST_SHELL)
        .stdin(Stdio::null())
        .stdout(Stdio::null());
    spawn_logged_host(home, &mut command)
}

#[tokio::test]
async fn session_reconcile_panic_is_isolated_and_host_remains_available() {
    let home = tempfile::tempdir().unwrap();
    let host = spawn_reconcile_panic_host(home.path());
    let harness = Harness::start_with_host(home, host).await;
    let terminal_id = harness
        .create_cat(RECONCILE_PANIC_SESSION_NAME)
        .await
        .manifest
        .terminal_id;

    let result = harness.client().reconcile().await.unwrap();

    assert_eq!(result.unreachable, EXPECTED_ISOLATED_PANICS);
    assert_eq!(result.reattached, usize::default());
    assert_eq!(result.exited, usize::default());
    assert_eq!(result.needs_recreate, usize::default());
    let status = harness.client().status().await.unwrap();
    assert_eq!(status.home, harness.home.path());
    let view = harness.client().get_session(terminal_id).await.unwrap();
    assert_eq!(view.manifest.terminal_id, terminal_id);
}
