use super::*;
use std::os::unix::fs::symlink;

const EARLY_EXIT_CODE: i32 = 73;
const EARLY_EXIT_MARKER: &str = "host-diagnostics-early-exit";
const EARLY_EXIT_DELAY_SECONDS: &str = "0.1";
const COMPETING_CANDIDATE_EXIT_CODE: i32 = 74;
const COMPETING_CANDIDATE_MARKER: &str = "host-diagnostics-competing-candidate";
const COMPETING_CANDIDATE_DELAY_SECONDS: &str = "0.5";
const HARD_KILL_MARKER: &str = "host-diagnostics-hard-kill";
const HOST_WAIT_SECONDS: &str = "30";
const LOG_AFTER: &[u8] = b"after-reopen";
const LOG_BEFORE: &[u8] = b"before-reopen:";
const LOG_SYMLINK_TARGET: &str = "symlink-target.log";
const NORMAL_STOP_MARKER: &str = "host-diagnostics-normal-stop";
const PROCESS_SCRIPT_ARGUMENT: &str = "-c";
const STDERR_MARKER_TIMEOUT: Duration = Duration::from_secs(2);
const STDERR_MARKER_POLL_INTERVAL: Duration = Duration::from_millis(10);
const TAIL_FILL_BYTE: u8 = b'x';
const TAIL_MARKER: &[u8] = b"host-diagnostics-tail-marker";
const TAIL_OVERFLOW_BYTES: usize = 128;

#[test]
fn host_diagnostics_rejects_final_component_symlink() {
    let home = tempfile::tempdir().unwrap();
    let target = home.path().join(LOG_SYMLINK_TARGET);
    std::fs::write(&target, b"").unwrap();
    symlink(target, home.path().join(HOST_STDERR_FILE_NAME)).unwrap();

    assert!(host_log::HostLog::open(home.path()).is_err());
}

#[test]
fn host_diagnostics_reopens_the_same_log_without_truncation() {
    let home = tempfile::tempdir().unwrap();
    let first = host_log::HostLog::open(home.path()).unwrap();
    first.append_for_test(LOG_BEFORE).unwrap();
    let second = host_log::HostLog::open(home.path()).unwrap();
    second.append_for_test(LOG_AFTER).unwrap();

    assert_eq!(first.identity(), second.identity());
    assert_eq!(second.tail().as_bytes(), [LOG_BEFORE, LOG_AFTER].concat());
}

#[test]
fn host_diagnostics_returns_only_the_bounded_log_tail() {
    let home = tempfile::tempdir().unwrap();
    let log = host_log::HostLog::open(home.path()).unwrap();
    let mut payload =
        vec![TAIL_FILL_BYTE; HOST_STDERR_MAX_TAIL_BYTES + TAIL_OVERFLOW_BYTES - TAIL_MARKER.len()];
    payload.extend_from_slice(TAIL_MARKER);
    log.append_for_test(&payload).unwrap();

    let tail = log.tail();
    assert_eq!(tail.len(), HOST_STDERR_MAX_TAIL_BYTES);
    assert!(tail.as_bytes().ends_with(TAIL_MARKER));
}

#[tokio::test]
async fn host_diagnostics_reports_early_exit_status_and_stderr() {
    let home = tempfile::tempdir().unwrap();
    let script = exit_script(EARLY_EXIT_MARKER, EARLY_EXIT_DELAY_SECONDS, EARLY_EXIT_CODE);
    let spawned = spawn_script(home.path(), &script);
    let mut host = HostGuard::new(spawned);

    let error = wait_for_host_result(home.path(), &mut host)
        .await
        .unwrap_err();

    assert!(error.contains(&EARLY_EXIT_CODE.to_string()), "{error}");
    assert!(error.contains(EARLY_EXIT_MARKER), "{error}");
}

#[tokio::test]
async fn host_readiness_rejects_a_competing_host_status() {
    let home = tempfile::tempdir().unwrap();
    let mut competing = HostGuard::new(spawn_host(home.path()));
    wait_for_host(home.path(), &mut competing).await;
    let script = exit_script(
        COMPETING_CANDIDATE_MARKER,
        COMPETING_CANDIDATE_DELAY_SECONDS,
        COMPETING_CANDIDATE_EXIT_CODE,
    );
    let mut candidate = HostGuard::new(spawn_script(home.path(), &script));
    wait_for_stderr_marker(&candidate, COMPETING_CANDIDATE_MARKER);

    let error = wait_for_host_result(home.path(), &mut candidate)
        .await
        .unwrap_err();

    assert!(
        error.contains(&COMPETING_CANDIDATE_EXIT_CODE.to_string()),
        "{error}"
    );
    assert!(error.contains(COMPETING_CANDIDATE_MARKER), "{error}");
}

#[test]
fn host_diagnostics_rejects_a_replacement_log_with_another_inode() {
    let first_home = tempfile::tempdir().unwrap();
    let second_home = tempfile::tempdir().unwrap();
    let script = waiting_script(NORMAL_STOP_MARKER, HOST_WAIT_SECONDS);
    let first = HostGuard::new(spawn_script(first_home.path(), &script));
    let second = spawn_script(second_home.path(), &script);

    assert!(first.require_same_log(&second).is_err());

    let _second = HostGuard::new(second);
}

#[test]
fn host_diagnostics_keeps_logs_readable_after_stop_and_kill() {
    assert_reaped_with_log(
        &waiting_script(NORMAL_STOP_MARKER, HOST_WAIT_SECONDS),
        NORMAL_STOP_MARKER,
        false,
    );
    assert_reaped_with_log(
        &waiting_script(HARD_KILL_MARKER, HOST_WAIT_SECONDS),
        HARD_KILL_MARKER,
        true,
    );
}

fn assert_reaped_with_log(script: &str, marker: &str, hard_kill: bool) {
    let home = tempfile::tempdir().unwrap();
    let mut host = HostGuard::new(spawn_script(home.path(), script));
    wait_for_stderr_marker(&host, marker);
    if hard_kill {
        host.kill_hard();
    } else {
        host.stop();
    }
    assert!(host.is_reaped());
    assert!(host.stderr_tail().contains(marker));
}

fn spawn_script(home: &Path, script: &str) -> SpawnedHost {
    let mut command = Command::new(TEST_SHELL);
    command
        .args([PROCESS_SCRIPT_ARGUMENT, script])
        .stdin(Stdio::null())
        .stdout(Stdio::null());
    spawn_logged_host(home, &mut command)
}

fn exit_script(marker: &str, delay_seconds: &str, exit_code: i32) -> String {
    format!("printf '{marker}\\n' >&2; sleep {delay_seconds}; exit {exit_code}")
}

fn waiting_script(marker: &str, delay_seconds: &str) -> String {
    format!("printf '{marker}\\n' >&2; exec /bin/sleep {delay_seconds}")
}

fn wait_for_stderr_marker(host: &HostGuard, marker: &str) {
    let deadline = Instant::now() + STDERR_MARKER_TIMEOUT;
    while Instant::now() < deadline {
        if host.stderr_tail().contains(marker) {
            return;
        }
        std::thread::sleep(STDERR_MARKER_POLL_INTERVAL);
    }
    panic!("Host stderr marker was not written: {marker}");
}
