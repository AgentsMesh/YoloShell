use super::*;
use yolo_runtime::host::secondary_custody_release_test_hook::{
    CustodyReleaseBoundary, CustodyReleaseTestBarrier,
};

const RELEASE_BARRIER_DIRECTORY: &str = "custody-release-barrier";
const BEFORE_DROP_SESSION_NAME: &str = "custody-release-before-drop";
const AFTER_DROP_SESSION_NAME: &str = "custody-release-after-drop";
const BARRIER_WAIT_DETAIL: &str = "Host did not reach the PTY custody release barrier";
const EXIT_WAIT_DETAIL: &str = "Worker did not publish its durable termination exit record";
const RUNTIME_GONE_WAIT_DETAIL: &str = "terminated PTY runtime identities remained live";
const TERMINATION_DISCONNECT_DETAIL: &str =
    "predecessor Host termination request unexpectedly survived SIGKILL";
const EXIT_REASON_MISMATCH_DETAIL: &str =
    "recovered PTY termination did not preserve its durable user intent";

#[tokio::test(flavor = "current_thread")]
async fn host_crash_before_platform_drop_recovers_exact_release() {
    run_release_recovery(
        CustodyReleaseBoundary::BeforePlatformDrop,
        BEFORE_DROP_SESSION_NAME,
    )
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn host_crash_after_platform_drop_recovers_exact_release() {
    run_release_recovery(
        CustodyReleaseBoundary::AfterPlatformDrop,
        AFTER_DROP_SESSION_NAME,
    )
    .await;
}

async fn run_release_recovery(phase: CustodyReleaseBoundary, session_name: &str) {
    let home = tempfile::tempdir().unwrap();
    let barrier_root = home.path().join(RELEASE_BARRIER_DIRECTORY);
    let predecessor = spawn_release_barrier_host(home.path(), &barrier_root);
    let mut harness = Harness::start_with_host(home, predecessor).await;
    let terminal_id = harness.create_cat(session_name).await.manifest.terminal_id;
    let paths = harness.paths();
    let runtime: RuntimeRecord = read_json(&paths.runtime_record(terminal_id)).unwrap();
    let predecessor_host = harness.host_identity();
    let barrier = CustodyReleaseTestBarrier::new(&barrier_root, terminal_id, phase).unwrap();
    barrier.arm().unwrap();

    let client = harness.client();
    let termination = tokio::spawn(async move { client.terminate(terminal_id, true).await });
    wait_for_condition(BARRIER_WAIT_DETAIL, || barrier.reached().unwrap()).await;

    let probe = barrier.probe().unwrap();
    assert_boundary_custody(phase, &probe, predecessor_host, &runtime);
    assert!(!paths.exit_record(terminal_id).exists());

    harness.kill_host_hard();
    assert_eq!(
        predecessor_host.observe(),
        process::ProcessIdentityObservation::Gone
    );
    let termination = timeout(PROCESS_EXIT_TIMEOUT, termination)
        .await
        .unwrap()
        .unwrap();
    assert!(termination.is_err(), "{TERMINATION_DISCONNECT_DETAIL}");

    harness.replace_host(spawn_host(harness.home.path())).await;
    let reconciliation = harness.client().reconcile().await.unwrap();
    assert_eq!(reconciliation.needs_recreate, 0);
    wait_for_condition(EXIT_WAIT_DETAIL, || paths.exit_record(terminal_id).exists()).await;

    let exit: yolo_runtime::ExitRecord = read_json(&paths.exit_record(terminal_id)).unwrap();
    let stored: RuntimeRecord = read_json(&paths.runtime_record(terminal_id)).unwrap();
    let tombstone = std::fs::read_to_string(paths.terminate_tombstone(terminal_id)).ok();
    let view = harness.client().get_session(terminal_id).await;
    assert_eq!(exit.runtime_epoch, runtime.runtime_epoch);
    assert_eq!(
        exit.reason,
        yolo_runtime::model::ExitReason::UserTerminate,
        "{EXIT_REASON_MISMATCH_DETAIL}: exit={exit:?}; runtime={stored:?}; \
         tombstone={tombstone:?}; view={view:?}; host_stderr={}",
        harness.host_stderr_tail()
    );
    assert_same_runtime_process(&stored, &runtime);
    wait_for_runtime_gone(&harness, &paths, terminal_id, &runtime).await;

    let successor_host = harness.host_identity();
    assert!(!probe.process_holds_pty_object(successor_host.pid).unwrap());
}

fn assert_boundary_custody(
    phase: CustodyReleaseBoundary,
    probe: &yolo_runtime::process::PtyMasterFdCustodyProbe,
    predecessor_host: ProcessIdentity,
    runtime: &RuntimeRecord,
) {
    let host_holds = probe
        .process_holds_pty_object(predecessor_host.pid)
        .unwrap();
    assert_eq!(
        host_holds,
        phase == CustodyReleaseBoundary::BeforePlatformDrop
    );
    assert!(!probe.process_holds_pty_object(runtime.worker_pid).unwrap());
    assert!(process::process_identity_matches(
        runtime.worker_pid,
        runtime.worker_start_time
    ));
    assert_eq!(
        process::observe_process_identity(runtime.child_pid, runtime.child_start_time),
        process::ProcessIdentityObservation::Gone
    );
}

fn spawn_release_barrier_host(home: &Path, barrier_root: &Path) -> SpawnedHost {
    let mut command = Command::new(runtime_binary());
    command
        .args(yolo_launch_contract::ProcessRole::Host.arguments())
        .args([cli::SERVE_COMMAND, cli::HOME_ARGUMENT])
        .arg(home)
        .env(
            CustodyReleaseTestBarrier::environment_variable(),
            barrier_root,
        )
        .env(SHELL_ENV, TEST_SHELL)
        .stdin(Stdio::null())
        .stdout(Stdio::null());
    spawn_logged_host(home, &mut command)
}

async fn wait_for_condition(detail: &'static str, mut condition: impl FnMut() -> bool) {
    let deadline = tokio::time::Instant::now() + PROCESS_EXIT_TIMEOUT;
    while !condition() && tokio::time::Instant::now() < deadline {
        tokio::time::sleep(PROCESS_POLL_INTERVAL).await;
    }
    assert!(condition(), "{detail}");
}

async fn wait_for_runtime_gone(
    harness: &Harness,
    paths: &Paths,
    terminal_id: Uuid,
    runtime: &RuntimeRecord,
) {
    let worker = ProcessIdentity {
        pid: runtime.worker_pid,
        start_time: runtime.worker_start_time,
    };
    let child = ProcessIdentity {
        pid: runtime.child_pid,
        start_time: runtime.child_start_time,
    };
    let deadline = tokio::time::Instant::now() + PROCESS_EXIT_TIMEOUT;
    loop {
        let worker_observation = worker.observe();
        let child_observation = child.observe();
        if worker_observation == process::ProcessIdentityObservation::Gone
            && child_observation == process::ProcessIdentityObservation::Gone
        {
            return;
        }
        if tokio::time::Instant::now() >= deadline {
            let stored = read_json::<RuntimeRecord>(&paths.runtime_record(terminal_id)).ok();
            let exit = read_json::<yolo_runtime::ExitRecord>(&paths.exit_record(terminal_id)).ok();
            let tombstone = std::fs::read_to_string(paths.terminate_tombstone(terminal_id)).ok();
            let custody = std::fs::read_to_string(paths.pty_custody(terminal_id)).ok();
            let worker_socket = paths
                .worker_socket(terminal_id)
                .map(|path| (path.clone(), path.exists()));
            panic!(
                "{RUNTIME_GONE_WAIT_DETAIL}: worker={worker:?} {worker_observation:?}; \
                 child={child:?} {child_observation:?}; runtime={stored:?}; exit={exit:?}; \
                 tombstone={tombstone:?}; custody={custody:?}; worker_socket={worker_socket:?}; \
                 host_stderr={}",
                harness.host_stderr_tail()
            );
        }
        tokio::time::sleep(PROCESS_POLL_INTERVAL).await;
    }
}

fn assert_same_runtime_process(actual: &RuntimeRecord, expected: &RuntimeRecord) {
    assert_eq!(actual.runtime_epoch, expected.runtime_epoch);
    assert_eq!(actual.worker_id, expected.worker_id);
    assert_eq!(actual.worker_pid, expected.worker_pid);
    assert_eq!(actual.worker_start_time, expected.worker_start_time);
    assert_eq!(actual.child_pid, expected.child_pid);
    assert_eq!(actual.child_start_time, expected.child_start_time);
}
