use crate::worker::tests::*;
const UNCERTAIN_EXEC_COMMAND: &str = "/bin/true";
const UNCERTAIN_EXEC_SCENARIO_ROLE_ENV: &str = "TEST_UNCERTAIN_EXEC_SCENARIO_ROLE";
const UNCERTAIN_EXEC_SCENARIO_ROLE: &str = "scenario";
const UNCERTAIN_EXEC_SCENARIO_MODE_ENV: &str = "TEST_UNCERTAIN_EXEC_SCENARIO_MODE";
const UNCERTAIN_EXEC_SCENARIO_STARTED_ENV: &str = "TEST_UNCERTAIN_EXEC_SCENARIO_STARTED";
const UNCERTAIN_EXEC_SCENARIO_COMPLETED_ENV: &str = "TEST_UNCERTAIN_EXEC_SCENARIO_COMPLETED";
const UNCERTAIN_EXEC_SCENARIO_TEST: &str =
    "worker::tests::runtime_exec_report_adversarial::uncertain_exec_runtime_scenario_process";
const UNCERTAIN_EXEC_SCENARIO_STARTED_FILE: &str = "scenario-started.json";
const UNCERTAIN_EXEC_SCENARIO_COMPLETED_FILE: &str = "scenario-completed.json";
const UNCERTAIN_EXEC_REPORT_MARKER_FILE: &str = "exec-report.json";
const UNCERTAIN_EXEC_PERSISTENCE_AUDIT_FILE: &str = "persistence-audit.json";
const UNCERTAIN_EXEC_NO_RETIREMENT_WINDOW: Duration = crate::session_timing::WORKER_AUTH_TIMEOUT;
const UNCERTAIN_EXEC_SCENARIO_TIMEOUT: Duration = UNCERTAIN_EXEC_HOST_WAIT_TIMEOUT
    .saturating_add(UNCERTAIN_EXEC_HOST_WAIT_TIMEOUT)
    .saturating_add(UNCERTAIN_EXEC_NO_RETIREMENT_WINDOW)
    .saturating_add(RUNTIME_ERROR_CUSTODY_TIMEOUT)
    .saturating_add(RUNTIME_ERROR_CUSTODY_TIMEOUT)
    .saturating_add(WORKER_TERMINATION_TIMEOUT);
const UNCERTAIN_EXEC_CANCELLATION_TIMEOUT_DETAIL: &str =
    "uncertain-exec Worker did not acknowledge task cancellation";

struct UncertainExecScenario(Option<std::process::Child>);
impl UncertainExecScenario {
    async fn run(mode: &str) {
        let directory = tempfile::tempdir().unwrap();
        let started = directory.path().join(UNCERTAIN_EXEC_SCENARIO_STARTED_FILE);
        let completed = directory
            .path()
            .join(UNCERTAIN_EXEC_SCENARIO_COMPLETED_FILE);
        let exec_report = directory.path().join(UNCERTAIN_EXEC_REPORT_MARKER_FILE);
        let audit = directory.path().join(UNCERTAIN_EXEC_PERSISTENCE_AUDIT_FILE);
        let child = crate::process::spawn_waitable_test_child(
            std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    crate::constants::TEST_HARNESS_EXACT_ARGUMENT,
                    UNCERTAIN_EXEC_SCENARIO_TEST,
                    crate::constants::TEST_HARNESS_NOCAPTURE_ARGUMENT,
                    crate::constants::TEST_HARNESS_SINGLE_THREAD_ARGUMENT,
                ])
                .env(
                    UNCERTAIN_EXEC_SCENARIO_ROLE_ENV,
                    UNCERTAIN_EXEC_SCENARIO_ROLE,
                )
                .env(UNCERTAIN_EXEC_SCENARIO_MODE_ENV, mode)
                .env(UNCERTAIN_EXEC_SCENARIO_STARTED_ENV, &started)
                .env(UNCERTAIN_EXEC_SCENARIO_COMPLETED_ENV, &completed)
                .env(
                    bootstrap_exec_test_hook::EXEC_REPORT_MARKER_ENV,
                    &exec_report,
                )
                .env(runtime_persistence_test_hook::AUDIT_PATH_ENV, &audit),
        );
        let mut scenario = Self(Some(child));
        let status = tokio::time::timeout(UNCERTAIN_EXEC_SCENARIO_TIMEOUT, async {
            loop {
                if let Some(status) = scenario.0.as_mut().unwrap().try_wait().unwrap() {
                    return status;
                }
                tokio::time::sleep(RUNTIME_ERROR_CUSTODY_POLL_INTERVAL).await;
            }
        })
        .await
        .expect(RUNTIME_ERROR_WORKER_TIMEOUT_DETAIL);
        assert!(status.success(), "{status}");
        let started_mode: String = read_json(&started).unwrap();
        let completed_mode: String = read_json(&completed).unwrap();
        assert_eq!(started_mode, mode);
        assert_eq!(completed_mode, mode);
        scenario.0 = None;
    }
}

impl Drop for UncertainExecScenario {
    fn drop(&mut self) {
        if let Some(child) = self.0.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl RuntimeErrorWorker {
    async fn cancel_and_drain(mut self) {
        let worker = self.0.take().unwrap();
        worker.abort();
        let cancellation = tokio::time::timeout(RUNTIME_ERROR_CUSTODY_TIMEOUT, worker)
            .await
            .expect(UNCERTAIN_EXEC_CANCELLATION_TIMEOUT_DETAIL)
            .unwrap_err();
        assert!(cancellation.is_cancelled());
    }
}

#[tokio::test]
async fn uncertain_exec_reports_preserve_recovery_evidence_without_rollback() {
    for mode in [
        bootstrap_exec_test_hook::EXEC_REPORT_MODE_MALFORMED,
        bootstrap_exec_test_hook::EXEC_REPORT_MODE_TIMEOUT,
    ] {
        UncertainExecScenario::run(mode).await;
    }
}

#[test]
fn uncertain_exec_runtime_scenario_process() {
    if std::env::var(UNCERTAIN_EXEC_SCENARIO_ROLE_ENV).as_deref()
        != Ok(UNCERTAIN_EXEC_SCENARIO_ROLE)
    {
        return;
    }
    let mode = std::env::var(UNCERTAIN_EXEC_SCENARIO_MODE_ENV).unwrap();
    publish_uncertain_exec_scenario_marker(UNCERTAIN_EXEC_SCENARIO_STARTED_ENV, &mode);
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(assert_uncertain_exec_report(&mode));
    publish_uncertain_exec_scenario_marker(UNCERTAIN_EXEC_SCENARIO_COMPLETED_ENV, &mode);
}

fn publish_uncertain_exec_scenario_marker(environment: &str, mode: &str) {
    let path = std::env::var_os(environment).unwrap();
    atomic_write_json(std::path::Path::new(&path), &mode).unwrap();
}

async fn assert_uncertain_exec_report(mode: &str) {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let terminal_id = Uuid::new_v4();
    let mut launch = launch_spec(&paths, terminal_id);
    launch.command = UNCERTAIN_EXEC_COMMAND.into();
    launch.environment.clear();
    launch.environment.insert(
        bootstrap_exec_test_hook::EXEC_REPORT_MODE_ENV.into(),
        mode.into(),
    );
    launch.environment.insert(
        bootstrap_exec_test_hook::EXEC_REPORT_MARKER_ENV.into(),
        std::env::var(bootstrap_exec_test_hook::EXEC_REPORT_MARKER_ENV).unwrap(),
    );
    let launch_path = paths.worker_launch(terminal_id);
    write_launch(&paths, terminal_id, &launch);
    runtime_result_test_hook::watch(terminal_id);
    let worker = RuntimeErrorWorker::spawn(paths.clone(), terminal_id);
    let host = UncertainExecHost::spawn(&paths, terminal_id);
    host.wait_for_no_decision().await;
    let diagnostic = runtime_result_test_hook::take(terminal_id)
        .expect(RUNTIME_ERROR_PRE_CLEANUP_PENDING_DETAIL);
    let expected = bootstrap_exec_test_hook::expected_failure_context(mode).unwrap();
    assert!(diagnostic.contains(expected), "{diagnostic}");

    let exec_report_path = std::env::var_os(bootstrap_exec_test_hook::EXEC_REPORT_MARKER_ENV)
        .map(std::path::PathBuf::from)
        .unwrap();
    assert_eq!(read_json::<String>(&exec_report_path).unwrap(), mode);

    let record: RuntimeRecord = read_json(&paths.runtime_record(terminal_id)).unwrap();
    assert_eq!(record.bootstrap_delivery, BootstrapDelivery::Pending);
    assert!(launch_path.exists());

    let child = process::ProcessIdentity {
        pid: record.child_pid,
        start_time: record.child_start_time,
    };
    host.release().await;
    let unexpectedly_finished = tokio::time::timeout(UNCERTAIN_EXEC_NO_RETIREMENT_WINDOW, async {
        while !worker.0.as_ref().unwrap().is_finished() {
            tokio::time::sleep(RUNTIME_ERROR_CUSTODY_POLL_INTERVAL).await;
        }
    })
    .await;
    assert!(unexpectedly_finished.is_err());
    assert_eq!(
        read_json::<RuntimeRecord>(&paths.runtime_record(terminal_id))
            .unwrap()
            .bootstrap_delivery,
        BootstrapDelivery::Pending
    );
    assert!(launch_path.exists());
    let audit_path = std::env::var_os(runtime_persistence_test_hook::AUDIT_PATH_ENV)
        .map(std::path::PathBuf::from)
        .unwrap();
    let events = runtime_persistence_test_hook::read(&audit_path).unwrap();
    assert!(
        events
            .iter()
            .any(|event| event.publishes(BootstrapDelivery::Pending))
    );
    assert!(
        !events
            .iter()
            .any(|event| event.publishes(BootstrapDelivery::Failed))
    );
    assert!(!events.iter().any(|event| event.removes(&launch_path)));
    worker.cancel_and_drain().await;
    wait_for_process_exit(child).await;
}
