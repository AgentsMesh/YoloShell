use crate::worker::tests::*;
const ORPHAN_EXIT_ISOLATION_ENV: &str = "YOLO_ORPHAN_EXIT_ISOLATION";
const ORPHAN_EXIT_ISOLATION_ROLE: &str = "run-orphan-worker";
const ORPHAN_EXIT_HOME_ENV: &str = "YOLO_ORPHAN_EXIT_HOME";
const ORPHAN_EXIT_SESSION_ENV: &str = "YOLO_ORPHAN_EXIT_SESSION";
const ORPHAN_EXIT_ISOLATED_TEST: &str =
    "worker::tests::runtime_exit_process_support::orphan_exit_barrier_isolated_worker";
const ORPHAN_EXIT_PROCESS_TIMEOUT: &str =
    "isolated orphan worker did not exit within its complete lifecycle budget";
const ORPHAN_EXIT_PROCESS_FAILURE: &str = "isolated orphan worker returned a failure status";
const ORPHAN_STARTUP_PROCESS_TIMEOUT: &str =
    "isolated orphan worker did not publish its runtime before the startup deadline";
const ORPHAN_STARTUP_EARLY_EXIT: &str =
    "isolated orphan worker exited before publishing its runtime";
const ORPHAN_STARTUP_CUSTODY_EXIT: &str =
    "isolated orphan worker Host custody ended before runtime publication";
const ORPHAN_STARTUP_TIMEOUT: Duration = crate::session_timing::WORKER_READY_TIMEOUT;
const ORPHAN_EXIT_LIFECYCLE_TIMEOUT: Duration = WORKER_TERMINATION_TIMEOUT
    .saturating_add(crate::session_timing::SESSION_LIFECYCLE_RESPONSE_SETTLEMENT_MARGIN);
const _: () =
    assert!(WORKER_TERMINATION_TIMEOUT.as_nanos() < ORPHAN_EXIT_LIFECYCLE_TIMEOUT.as_nanos());
include!("../../test_support/integration/runtime_exit/runtime_exit_process_diagnostics.rs");
include!("../../test_support/integration/runtime_exit/runtime_exit_process_namespace.rs");
#[test]
fn orphan_worker_process_returns_after_durable_exit() {
    let directory = GuardianRuntimeDirectory::new();
    let mut socket_root = crate::paths::IsolatedTestSocketRoot::create().unwrap();
    let paths = Paths::new_with_socket_root(directory.path(), socket_root.path().to_path_buf());
    let terminal_id = Uuid::new_v4();
    let mut launch = launch_spec(&paths, terminal_id);
    launch.command = FIXTURE_COMMAND.into();
    launch.environment.clear();
    write_launch(&paths, terminal_id, &launch);
    let mut custody = Some(spawn_active_runtime_custody_thread(
        paths.clone(),
        terminal_id,
    ));
    let mut child_stderr = tempfile::tempfile().unwrap();
    let child_stderr_sink = child_stderr.try_clone().unwrap();
    let mut child = crate::process::spawn_waitable_test_child(
        std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                crate::constants::TEST_HARNESS_EXACT_ARGUMENT,
                ORPHAN_EXIT_ISOLATED_TEST,
                crate::constants::TEST_HARNESS_NOCAPTURE_ARGUMENT,
                crate::constants::TEST_HARNESS_SINGLE_THREAD_ARGUMENT,
            ])
            .env(ORPHAN_EXIT_ISOLATION_ENV, ORPHAN_EXIT_ISOLATION_ROLE)
            .env(ORPHAN_EXIT_HOME_ENV, paths.home())
            .env(ORPHAN_EXIT_SESSION_ENV, terminal_id.to_string())
            .env(crate::paths::SOCKET_ROOT_ENVIRONMENT, socket_root.path())
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::from(child_stderr_sink)),
    );
    let runtime = wait_for_orphan_runtime(
        &mut child,
        &paths,
        terminal_id,
        &mut custody,
        &mut child_stderr,
    );
    assert!(child.try_wait().unwrap().is_none());
    #[cfg(target_os = "macos")]
    assert_orphan_guardian_uses_explicit_namespace(&paths, terminal_id);
    std::fs::remove_file(paths.manifest(terminal_id)).unwrap();
    let status = wait_for_orphan_worker(
        &mut child,
        &paths,
        terminal_id,
        &mut custody,
        &mut child_stderr,
    );
    if !status.success() {
        fail_orphan_exit(
            ORPHAN_EXIT_PROCESS_FAILURE,
            Some(status),
            &mut child,
            &paths,
            terminal_id,
            &mut custody,
            &mut child_stderr,
        );
    }
    custody.take().unwrap().join().unwrap().unwrap();

    let exit: ExitRecord = read_json(&paths.exit_record(terminal_id)).unwrap();
    let journal = Journal::open(paths.journal(terminal_id), runtime.runtime_epoch).unwrap();
    assert_eq!(exit.runtime_epoch, runtime.runtime_epoch);
    assert_eq!(exit.last_output_sequence, journal.last_sequence());
    assert_eq!(exit.reason, ExitReason::Lost);
    drop(journal);
    directory.close(&paths).unwrap();
    socket_root.finalize().unwrap();
}

fn wait_for_orphan_runtime(
    child: &mut std::process::Child,
    paths: &Paths,
    terminal_id: Uuid,
    custody: &mut Option<std::thread::JoinHandle<Result<()>>>,
    child_stderr: &mut std::fs::File,
) -> RuntimeRecord {
    let deadline = Instant::now() + ORPHAN_STARTUP_TIMEOUT;
    loop {
        if let Ok(runtime) = read_json::<RuntimeRecord>(&paths.runtime_record(terminal_id))
            && runtime.bootstrap_delivery.is_ready()
            && matches!(
                startup_checkpoint::worker_startup_checkpoint_allows_lifecycle_mutation(
                    paths,
                    terminal_id,
                    Some(&runtime),
                ),
                Ok(true)
            )
        {
            return runtime;
        }
        if let Some(status) = child.try_wait().unwrap() {
            fail_orphan_startup(
                ORPHAN_STARTUP_EARLY_EXIT,
                Some(status),
                child,
                paths,
                terminal_id,
                custody,
                child_stderr,
            );
        }
        if custody.as_ref().is_some_and(|thread| thread.is_finished()) {
            fail_orphan_startup(
                ORPHAN_STARTUP_CUSTODY_EXIT,
                None,
                child,
                paths,
                terminal_id,
                custody,
                child_stderr,
            );
        }
        if Instant::now() >= deadline {
            fail_orphan_startup(
                ORPHAN_STARTUP_PROCESS_TIMEOUT,
                None,
                child,
                paths,
                terminal_id,
                custody,
                child_stderr,
            );
        }
        std::thread::sleep(TEST_READY_POLL_INTERVAL);
    }
}

fn wait_for_orphan_worker(
    child: &mut std::process::Child,
    paths: &Paths,
    terminal_id: Uuid,
    custody: &mut Option<std::thread::JoinHandle<Result<()>>>,
    child_stderr: &mut std::fs::File,
) -> std::process::ExitStatus {
    let deadline = Instant::now() + ORPHAN_EXIT_LIFECYCLE_TIMEOUT;
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            return status;
        }
        if Instant::now() >= deadline {
            fail_orphan_exit(
                ORPHAN_EXIT_PROCESS_TIMEOUT,
                None,
                child,
                paths,
                terminal_id,
                custody,
                child_stderr,
            );
        }
        std::thread::sleep(TEST_READY_POLL_INTERVAL);
    }
}

#[test]
fn orphan_exit_barrier_isolated_worker() {
    if std::env::var(ORPHAN_EXIT_ISOLATION_ENV).as_deref() != Ok(ORPHAN_EXIT_ISOLATION_ROLE) {
        return;
    }
    let home = std::env::var_os(ORPHAN_EXIT_HOME_ENV).unwrap();
    let terminal_id = std::env::var(ORPHAN_EXIT_SESSION_ENV)
        .unwrap()
        .parse()
        .unwrap();
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(run(Paths::new(home), terminal_id))
        .unwrap();
}
