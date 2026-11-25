use crate::worker::tests::*;
const RUNTIME_ERROR_WATCHDOG_HELPER_ENV: &str = "YOLO_RUNTIME_ERROR_WATCHDOG_HELPER";
const RUNTIME_ERROR_WATCHDOG_HELPER_ROLE: &str = "descendant-holder";
const RUNTIME_ERROR_WATCHDOG_MARKER_ENV: &str = "YOLO_RUNTIME_ERROR_WATCHDOG_MARKER";
const RUNTIME_ERROR_WATCHDOG_MARKER_FILE: &str = "descendant.json";
const RUNTIME_ERROR_WATCHDOG_HELPER_TEST: &str = "worker::tests::runtime_error_process_watchdog_adversarial::concurrent_runtime_error_watchdog_process_helper";
const RUNTIME_ERROR_WATCHDOG_SLEEP_EXECUTABLE: &str = "/bin/sleep";
const RUNTIME_ERROR_WATCHDOG_SLEEP_ARGUMENT: &str = "60";
const RUNTIME_ERROR_WATCHDOG_EXPECTED_FAILURE_DETAIL: &str =
    "expired concurrent runtime-error watchdog unexpectedly succeeded";
const RUNTIME_ERROR_WATCHDOG_MARKER_TIMEOUT_DETAIL: &str =
    "runtime-error watchdog descendant was not published";
const RUNTIME_ERROR_WATCHDOG_ROOT_RETAINED_DETAIL: &str =
    "runtime-error watchdog retained its helper root";
const RUNTIME_ERROR_WATCHDOG_DESCENDANT_RETAINED_DETAIL: &str =
    "runtime-error watchdog retained its helper descendant";

#[derive(Deserialize, Serialize)]
pub(super) struct RuntimeErrorWatchdogProcessRecord {
    pub(super) pid: u32,
    pub(super) start_time: u64,
}

#[test]
fn runtime_error_process_watchdog_terminates_helper_descendants() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let marker = directory.path().join(RUNTIME_ERROR_WATCHDOG_MARKER_FILE);
    let mut command =
        ConcurrentRuntimeErrorProcess::test_command(RUNTIME_ERROR_WATCHDOG_HELPER_TEST)?;
    command
        .env(
            RUNTIME_ERROR_WATCHDOG_HELPER_ENV,
            RUNTIME_ERROR_WATCHDOG_HELPER_ROLE,
        )
        .env(RUNTIME_ERROR_WATCHDOG_MARKER_ENV, &marker);
    let mut process = ConcurrentRuntimeErrorProcess::spawn_command(command)?;
    let root = process
        .identity
        .context(RUNTIME_ERROR_PROCESS_IDENTITY_DETAIL)?;
    let descendant = wait_for_runtime_error_watchdog_descendant(&marker)?;
    process.deadline = Instant::now();
    let error = process
        .finish()
        .expect_err(RUNTIME_ERROR_WATCHDOG_EXPECTED_FAILURE_DETAIL);
    anyhow::ensure!(
        error
            .to_string()
            .contains(RUNTIME_ERROR_PROCESS_TIMEOUT_DETAIL),
        error
    );
    anyhow::ensure!(!root.matches(), RUNTIME_ERROR_WATCHDOG_ROOT_RETAINED_DETAIL);
    anyhow::ensure!(
        !descendant.matches(),
        RUNTIME_ERROR_WATCHDOG_DESCENDANT_RETAINED_DETAIL
    );
    Ok(())
}

pub(super) fn wait_for_runtime_error_watchdog_descendant(
    marker: &std::path::Path,
) -> Result<process::ProcessIdentity> {
    let deadline = Instant::now() + RUNTIME_ERROR_RECORD_TIMEOUT;
    loop {
        if let Ok(record) = read_json::<RuntimeErrorWatchdogProcessRecord>(marker) {
            return Ok(process::ProcessIdentity {
                pid: record.pid,
                start_time: record.start_time,
            });
        }
        anyhow::ensure!(
            Instant::now() < deadline,
            RUNTIME_ERROR_WATCHDOG_MARKER_TIMEOUT_DETAIL
        );
        std::thread::sleep(RUNTIME_ERROR_CUSTODY_POLL_INTERVAL);
    }
}

#[test]
fn concurrent_runtime_error_watchdog_process_helper() {
    if std::env::var(RUNTIME_ERROR_WATCHDOG_HELPER_ENV).as_deref()
        != Ok(RUNTIME_ERROR_WATCHDOG_HELPER_ROLE)
    {
        return;
    }
    let marker = std::env::var_os(RUNTIME_ERROR_WATCHDOG_MARKER_ENV).unwrap();
    let child = crate::process::spawn_waitable_test_child(
        std::process::Command::new(RUNTIME_ERROR_WATCHDOG_SLEEP_EXECUTABLE)
            .arg(RUNTIME_ERROR_WATCHDOG_SLEEP_ARGUMENT),
    );
    let identity = process::ProcessIdentity::capture(child.id()).unwrap();
    let record = RuntimeErrorWatchdogProcessRecord {
        pid: identity.pid,
        start_time: identity.start_time,
    };
    atomic_write_json(std::path::Path::new(&marker), &record).unwrap();
    loop {
        std::thread::sleep(RUNTIME_ERROR_CUSTODY_TIMEOUT);
    }
}
