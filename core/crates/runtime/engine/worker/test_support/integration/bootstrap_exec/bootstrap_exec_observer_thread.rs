use super::super::*;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::time::Duration;

const HELPER_ENV: &str = "YOLO_TEST_NON_LEADER_EXEC";
const HELPER_TEST: &str =
    "worker::bootstrap::bootstrap_exec_observer::platform::thread_tests::non_leader_exec_helper";
const THREAD_FILE: &str = "thread-id";
const RELEASE_FILE: &str = "release";
const TIMEOUT: Duration = Duration::from_secs(5);
const POLL_INTERVAL: Duration = Duration::from_millis(5);
const WAIT_DETAIL: &str = "non-leader exec fixture did not reach its next boundary";

pub(super) struct Child {
    process: std::process::Child,
    directory: tempfile::TempDir,
    waited: bool,
}

impl Child {
    pub(super) fn spawn() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let process = crate::process::spawn_waitable_test_child(
            Command::new(std::env::current_exe().unwrap())
                .args([
                    BOOTSTRAP_TEST_EXACT_ARGUMENT,
                    HELPER_TEST,
                    BOOTSTRAP_TEST_NOCAPTURE_ARGUMENT,
                ])
                .env(HELPER_ENV, directory.path())
                .stdin(Stdio::null())
                .stdout(Stdio::null()),
        );
        Self {
            process,
            directory,
            waited: false,
        }
    }

    pub(super) fn target(&mut self) -> BootstrapExecTarget {
        let ready = self.directory.path().join(THREAD_FILE);
        let deadline = deadline();
        loop {
            if let Ok(value) = std::fs::read_to_string(&ready)
                && let Ok(thread_id) = value.parse()
            {
                return BootstrapExecTarget {
                    process: ProcessIdentity::capture(self.process.id()).unwrap(),
                    thread_id,
                };
            }
            assert!(self.process.try_wait().unwrap().is_none(), "{WAIT_DETAIL}");
            assert!(Instant::now() < deadline, "{WAIT_DETAIL}");
            std::thread::sleep(POLL_INTERVAL);
        }
    }

    pub(super) fn release(&self) {
        std::fs::write(self.directory.path().join(RELEASE_FILE), []).unwrap();
    }

    pub(super) fn wait(&mut self) -> ExitStatus {
        let deadline = deadline();
        loop {
            if let Some(status) = self.process.try_wait().unwrap() {
                self.waited = true;
                return status;
            }
            assert!(Instant::now() < deadline, "{WAIT_DETAIL}");
            std::thread::sleep(POLL_INTERVAL);
        }
    }
}

impl Drop for Child {
    fn drop(&mut self) {
        if !self.waited {
            let _ = self.process.kill();
            let _ = self.process.wait();
        }
    }
}

pub(super) fn deadline() -> Instant {
    Instant::now() + TIMEOUT
}

pub(super) fn run_helper() {
    let Some(directory) = std::env::var_os(HELPER_ENV).map(PathBuf::from) else {
        return;
    };
    let thread_id = crate::worker::bootstrap_exec_observer::current_exec_thread_id().unwrap();
    assert_ne!(thread_id, std::process::id());
    std::fs::write(directory.join(THREAD_FILE), thread_id.to_string()).unwrap();
    wait_for_release(&directory.join(RELEASE_FILE));
    let error = Command::new(crate::test_executables::TRUE).exec();
    panic!("non-leader exec failed: {error}");
}

fn wait_for_release(path: &Path) {
    let deadline = deadline();
    while !path.exists() {
        assert!(Instant::now() < deadline, "{WAIT_DETAIL}");
        std::thread::sleep(POLL_INTERVAL);
    }
}
