use crate::constants::{TEST_HARNESS_EXACT_ARGUMENT, TEST_HARNESS_QUIET_ARGUMENT};
use crate::custody_guardian::test_child_guard::GuardedChild;
use anyhow::{Context, Result, ensure};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;

const HELPER_TEST: &str = "custody_guardian::settlement_failure::containment_tests::\
escaped_holder_tests::watcher::reopen_watcher_helper";
const ROLE_ENV: &str = "YOLO_GUARDIAN_REOPEN_WATCHER_ROLE";
const READY_ENV: &str = "YOLO_GUARDIAN_REOPEN_WATCHER_READY";
const TRIGGER_ENV: &str = "YOLO_GUARDIAN_REOPEN_WATCHER_TRIGGER";
const RESULT_ENV: &str = "YOLO_GUARDIAN_REOPEN_WATCHER_RESULT";
const PTY_PATH_ENV: &str = "YOLO_GUARDIAN_REOPEN_WATCHER_PTY";
const WATCHER_ROLE: &str = "reopen-watcher";
const READY_FILE: &str = "watcher.ready";
const TRIGGER_FILE: &str = "watcher.trigger";
const RESULT_FILE: &str = "watcher.result";
const REOPEN_REJECTED: &str = "permission-denied";
const REOPEN_ACCEPTED: &str = "reopen-accepted";
const REOPEN_UNEXPECTED_ERROR: &str = "unexpected-open-error";
const EMPTY: &[u8] = b"";
const OPEN_FLAGS: libc::c_int =
    libc::O_NOCTTY | libc::O_NONBLOCK | libc::O_CLOEXEC | libc::O_NOFOLLOW;
const WATCHER_READY_TIMEOUT: std::time::Duration = crate::session_timing::WORKER_READY_TIMEOUT;
const WATCHER_RESULT_TIMEOUT: std::time::Duration =
    crate::session_timing::WORKER_TERMINATION_PROTOCOL_TIMEOUT;
const WATCHER_POLL: std::time::Duration = crate::session_timing::GRACEFUL_TERMINATION_POLL_INTERVAL;
const WATCHER_TIMEOUT_DETAIL: &str = "independent PTY reopen watcher did not converge";
const WATCHER_RESULT_DETAIL: &str = "sealed PTY slave was not proven to reject reopen";
const WATCHER_EXIT_DETAIL: &str = "independent PTY reopen watcher failed";

pub(super) struct ReopenWatcher {
    _directory: tempfile::TempDir,
    child: GuardedChild,
    trigger: PathBuf,
    result: PathBuf,
}

impl ReopenWatcher {
    pub(super) fn spawn(pty_path: &Path) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let ready = directory.path().join(READY_FILE);
        let trigger = directory.path().join(TRIGGER_FILE);
        let result = directory.path().join(RESULT_FILE);
        let child = GuardedChild::spawn(|| {
            crate::process::spawn_waitable_test_child(
                Command::new(std::env::current_exe().unwrap())
                    .args([
                        TEST_HARNESS_EXACT_ARGUMENT,
                        HELPER_TEST,
                        TEST_HARNESS_QUIET_ARGUMENT,
                    ])
                    .env(ROLE_ENV, WATCHER_ROLE)
                    .env(READY_ENV, &ready)
                    .env(TRIGGER_ENV, &trigger)
                    .env(RESULT_ENV, &result)
                    .env(PTY_PATH_ENV, pty_path)
                    .stdout(Stdio::null())
                    .stderr(Stdio::null()),
            )
        });
        let watcher = Self {
            _directory: directory,
            child,
            trigger,
            result,
        };
        wait_for_path(&ready, WATCHER_READY_TIMEOUT).unwrap();
        watcher
    }

    pub(super) fn prove_reopen_rejected(mut self) -> Result<()> {
        std::fs::write(&self.trigger, EMPTY)?;
        self.wait_for_exit()?;
        let result = std::fs::read_to_string(&self.result)?;
        ensure!(
            result == REOPEN_REJECTED,
            "{WATCHER_RESULT_DETAIL}: {result}"
        );
        Ok(())
    }

    fn wait_for_exit(&mut self) -> Result<()> {
        if self.child.is_complete() {
            return Ok(());
        }
        let deadline = Instant::now() + WATCHER_RESULT_TIMEOUT;
        loop {
            if let Some(status) = self.child.try_wait()? {
                ensure!(status.success(), "{WATCHER_EXIT_DETAIL}: {status}");
                return Ok(());
            }
            ensure!(Instant::now() < deadline, WATCHER_TIMEOUT_DETAIL);
            std::thread::sleep(WATCHER_POLL);
        }
    }
}

#[test]
fn reopen_watcher_helper() -> Result<()> {
    if std::env::var(ROLE_ENV).as_deref() != Ok(WATCHER_ROLE) {
        return Ok(());
    }
    let ready = required_path(READY_ENV)?;
    let trigger = required_path(TRIGGER_ENV)?;
    let result = required_path(RESULT_ENV)?;
    std::fs::write(ready, EMPTY)?;
    wait_for_path(&trigger, WATCHER_READY_TIMEOUT)?;
    let observed = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(OPEN_FLAGS)
        .open(required_path(PTY_PATH_ENV)?);
    let value = match observed {
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => REOPEN_REJECTED,
        Ok(_) => REOPEN_ACCEPTED,
        Err(_) => REOPEN_UNEXPECTED_ERROR,
    };
    std::fs::write(result, value)?;
    Ok(())
}

fn wait_for_path(path: &Path, timeout: std::time::Duration) -> Result<()> {
    let deadline = Instant::now() + timeout;
    while !path.exists() {
        ensure!(Instant::now() < deadline, WATCHER_TIMEOUT_DETAIL);
        std::thread::sleep(WATCHER_POLL);
    }
    Ok(())
}

fn required_path(name: &str) -> Result<PathBuf> {
    std::env::var_os(name)
        .map(PathBuf::from)
        .context(WATCHER_TIMEOUT_DETAIL)
}
