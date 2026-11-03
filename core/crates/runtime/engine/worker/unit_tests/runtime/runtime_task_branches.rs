
fn monitor_runtime() -> (
    std::sync::Arc<crate::agent::AgentRegistry>,
    std::sync::Arc<yolo_agent_providers::ProviderRuntimeRegistry>,
) {
    let provider_runtime = std::sync::Arc::new(yolo_agent_providers::builtin_provider_runtime());
    let registry = std::sync::Arc::new(crate::agent::agent_registry_from_runtime(
        &provider_runtime,
    ));
    (registry, provider_runtime)
}

use crate::worker::tests::*;
const ACCEPT_FAILURE_DETAIL: &str = "injected accept failure";
const MONITOR_FAILURE_DETAIL: &str = "injected monitor failure";

pub(super) struct TaskDropProbe(pub(super) Arc<AtomicBool>);

impl Drop for TaskDropProbe {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}

fn monitor_config(fixture: &Fixture) -> AgentMonitorConfig {
    let home = fixture.state.paths.home().to_path_buf();
    let (registry, provider_runtime) = monitor_runtime();
    AgentMonitorConfig {
        registry,
        provider_runtime,
        context: DetectionContext::new(
            fixture.terminal_id(),
            home.clone(),
            [
                (
                    crate::agent::AgentKind::Codex,
                    home.join(crate::agent::codex::constants::DEFAULT_DATA_HOME_DIR),
                ),
                (
                    crate::agent::AgentKind::ClaudeCode,
                    home.join(crate::agent::claude::constants::DEFAULT_DATA_HOME_DIR),
                ),
            ],
        ),
        binding_path: fixture.state.paths.agent_binding(fixture.terminal_id()),
    }
}

#[test]
fn accept_failure_is_counted_and_schedules_backoff() {
    let fixture = Fixture::new();
    let mut tasks = JoinSet::new();
    let mut failures = 0;
    let clients = Arc::new(Semaphore::new(MAX_WORKER_CLIENTS));
    let delay = handle_accept_result(
        Err(io::Error::new(
            io::ErrorKind::ConnectionAborted,
            ACCEPT_FAILURE_DETAIL,
        )),
        &mut tasks,
        &fixture.state,
        &clients,
        &mut failures,
    );
    assert_eq!(failures, 1);
    assert!(tasks.is_empty());
    assert!(delay.is_some());
}

#[tokio::test(start_paused = true)]
async fn accept_backoff_wait_is_immediately_interruptible() {
    let mut retry = Some(Box::pin(tokio::time::sleep(SUPERVISION_RESTART_MAX_DELAY)));
    let shutdown = Notify::new();
    shutdown.notify_one();
    let interrupted = tokio::select! {
        _ = wait_for_accept_retry(&mut retry) => false,
        _ = shutdown.notified() => true,
    };
    assert!(interrupted);
}

#[tokio::test]
async fn task_shutdown_waits_for_cancelled_futures_to_release_state() {
    let dropped = Arc::new(AtomicBool::new(false));
    let probe = TaskDropProbe(Arc::clone(&dropped));
    let mut tasks = JoinSet::new();
    tasks.spawn(async move {
        let _probe = probe;
        std::future::pending::<()>().await;
    });
    tokio::task::yield_now().await;

    abort_and_drain_tasks(&mut tasks).await;

    assert!(tasks.is_empty());
    assert!(dropped.load(Ordering::Acquire));
}

#[test]
fn empty_task_completions_have_no_supervision_side_effects() {
    let fixture = Fixture::new();
    let mut tasks = JoinSet::new();
    handle_task_completion(None, &mut tasks, &fixture.state, None, UNUSED_SHELL_PID);
    handle_task_completion(
        Some(Ok(None)),
        &mut tasks,
        &fixture.state,
        None,
        UNUSED_SHELL_PID,
    );
    assert!(tasks.is_empty());
}

#[tokio::test]
async fn monitor_failure_restarts_only_with_live_child_and_configuration() {
    let fixture = Fixture::new();
    let config = monitor_config(&fixture);
    let completion = || {
        Some(Ok(Some(supervision::TaskFailure::message(
            supervision::AGENT_MONITOR_TASK,
            MONITOR_FAILURE_DETAIL,
        ))))
    };
    let mut tasks = JoinSet::new();
    handle_task_completion(
        completion(),
        &mut tasks,
        &fixture.state,
        Some(&config),
        UNUSED_SHELL_PID,
    );
    assert_eq!(tasks.len(), 1);
    tasks.abort_all();
    while tasks.join_next().await.is_some() {}

    handle_task_completion(
        completion(),
        &mut tasks,
        &fixture.state,
        None,
        UNUSED_SHELL_PID,
    );
    assert!(tasks.is_empty());
    fixture.state.child_waited.store(true, Ordering::Release);
    handle_task_completion(
        completion(),
        &mut tasks,
        &fixture.state,
        Some(&config),
        UNUSED_SHELL_PID,
    );
    assert!(tasks.is_empty());
}

include!("runtime_exit_barrier_tests.rs");
