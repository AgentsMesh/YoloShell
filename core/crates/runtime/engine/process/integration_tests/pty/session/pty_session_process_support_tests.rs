use crate::process::constants::PTY_SESSION_CAPABILITY_ENV;
use crate::runtime_constants::{TEST_HARNESS_EXACT_ARGUMENT, TEST_HARNESS_NOCAPTURE_ARGUMENT};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const HELPER_TEST: &str =
    "pty_session_proof::macos_scan::process_test_support::ready_process_helper";
const HELPER_ROLE_ENV: &str = "YOLO_PTY_PROOF_HELPER_ROLE";
const HELPER_READY_ENV: &str = "YOLO_PTY_PROOF_HELPER_READY";
const HELPER_ROLE: &str = "capability-holder";
const READY_MARKER: &str = "ready";
const EMPTY_MARKER: &[u8] = b"";
const READY_TIMEOUT: Duration = Duration::from_secs(5);
const READY_POLL_INTERVAL: Duration = Duration::from_millis(10);
const HELPER_READY_TIMEOUT_DETAIL: &str = "PTY proof helper did not become ready";

pub(in super::super) struct ReadyProcess {
    _directory: tempfile::TempDir,
    child: Child,
}

impl ReadyProcess {
    pub(in super::super) fn spawn(capability: &str, stdin: Stdio) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let ready = directory.path().join(READY_MARKER);
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args([
                TEST_HARNESS_EXACT_ARGUMENT,
                HELPER_TEST,
                TEST_HARNESS_NOCAPTURE_ARGUMENT,
            ])
            .env(HELPER_ROLE_ENV, HELPER_ROLE)
            .env(HELPER_READY_ENV, &ready)
            .env(PTY_SESSION_CAPABILITY_ENV, capability)
            .stdin(stdin)
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let child = crate::process::spawn_waitable_test_child(command);
        wait_for_ready(&ready);
        Self {
            _directory: directory,
            child,
        }
    }

    pub(in super::super) fn id(&self) -> u32 {
        self.child.id()
    }
}

impl Drop for ReadyProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn ready_process_helper() {
    if std::env::var(HELPER_ROLE_ENV).as_deref() != Ok(HELPER_ROLE) {
        return;
    }
    std::fs::write(required_path(HELPER_READY_ENV), EMPTY_MARKER).unwrap();
    loop {
        std::thread::park();
    }
}

fn required_path(name: &str) -> PathBuf {
    std::env::var_os(name).map(PathBuf::from).unwrap()
}

fn wait_for_ready(path: &Path) {
    let deadline = Instant::now() + READY_TIMEOUT;
    while !path.exists() {
        assert!(Instant::now() < deadline, "{HELPER_READY_TIMEOUT_DETAIL}");
        std::thread::sleep(READY_POLL_INTERVAL);
    }
}
