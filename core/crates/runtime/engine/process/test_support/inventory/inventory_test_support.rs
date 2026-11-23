use super::inventory_snapshot::InventoryEntry;
use super::{DetachedStandardError, ProcessIdentity, spawn_detached};
#[cfg(target_os = "macos")]
use crate::process::MacosProcessCapabilityChildWriter;
#[cfg(target_os = "macos")]
use crate::runtime_constants::{TEST_HARNESS_EXACT_ARGUMENT, TEST_HARNESS_NOCAPTURE_ARGUMENT};
use std::path::Path;
#[cfg(target_os = "macos")]
use std::path::PathBuf;
#[cfg(target_os = "macos")]
use std::process::{Command, Stdio};
#[cfg(target_os = "macos")]
use std::time::{Duration, Instant};

const TEST_SLEEP_EXECUTABLE: &str = "/bin/sleep";
const TEST_SLEEP_DURATION: &str = "30";
pub(super) const NO_PARENT_PID: u32 = 0;
#[cfg(target_os = "macos")]
const ENVIRONMENT_HELPER_TEST: &str =
    "inventory_test_support::tests::environment_process_helper_entrypoint";
#[cfg(target_os = "macos")]
const ENVIRONMENT_HELPER_ROLE_ENV: &str = "YOLO_INVENTORY_HELPER_ROLE";
#[cfg(target_os = "macos")]
const ENVIRONMENT_HELPER_READY_ENV: &str = "YOLO_INVENTORY_HELPER_READY";
#[cfg(target_os = "macos")]
const ENVIRONMENT_HELPER_ROLE: &str = "environment-holder";
#[cfg(target_os = "macos")]
const ENVIRONMENT_HELPER_READY_FILE: &str = "ready";
#[cfg(target_os = "macos")]
const ENVIRONMENT_HELPER_MARKER: &[u8] = b"";
#[cfg(target_os = "macos")]
const ENVIRONMENT_HELPER_TIMEOUT: Duration = Duration::from_secs(5);
#[cfg(target_os = "macos")]
const ENVIRONMENT_HELPER_POLL: Duration = Duration::from_millis(10);
#[cfg(target_os = "macos")]
const ENVIRONMENT_HELPER_TIMEOUT_DETAIL: &str = "inventory helper did not become ready";

#[cfg(target_os = "macos")]
#[path = "inventory_discovery_test_process.rs"]
mod discovery_process;

pub(super) struct TestProcess {
    child: std::process::Child,
    pub(super) identity: ProcessIdentity,
    _directory: Option<tempfile::TempDir>,
}

impl TestProcess {
    pub(super) fn spawn() -> Self {
        let child = spawn_detached(
            Path::new(TEST_SLEEP_EXECUTABLE),
            &[TEST_SLEEP_DURATION.into()],
            DetachedStandardError::Null,
        )
        .unwrap();
        let identity = ProcessIdentity::capture(child.id()).unwrap();
        Self {
            child,
            identity,
            _directory: None,
        }
    }

    pub(super) fn terminate(&mut self) {
        let _ = self.identity.signal(libc::SIGKILL);
        self.child.wait().unwrap();
    }

    #[cfg(target_os = "macos")]
    pub(super) fn spawn_with_environment(name: &str, value: &str) -> Self {
        Self::spawn_environment_holder(name, value, None)
    }

    #[cfg(target_os = "macos")]
    pub(super) fn spawn_with_environment_and_process_capability(
        name: &str,
        value: &str,
        writer: &MacosProcessCapabilityChildWriter,
    ) -> Self {
        Self::spawn_environment_holder(name, value, Some(writer))
    }

    #[cfg(target_os = "macos")]
    fn spawn_environment_holder(
        name: &str,
        value: &str,
        writer: Option<&MacosProcessCapabilityChildWriter>,
    ) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let ready = directory.path().join(ENVIRONMENT_HELPER_READY_FILE);
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args([
                TEST_HARNESS_EXACT_ARGUMENT,
                ENVIRONMENT_HELPER_TEST,
                TEST_HARNESS_NOCAPTURE_ARGUMENT,
            ])
            .env(ENVIRONMENT_HELPER_ROLE_ENV, ENVIRONMENT_HELPER_ROLE)
            .env(ENVIRONMENT_HELPER_READY_ENV, &ready)
            .env(name, value)
            .stderr(Stdio::null());
        if let Some(writer) = writer {
            writer.configure_exec_inheritance(&mut command);
        }
        let child = super::spawn_detached_with_control_pipes(command).unwrap();
        let identity = ProcessIdentity::capture(child.id()).unwrap();
        wait_for_environment_helper(&ready);
        Self {
            child,
            identity,
            _directory: Some(directory),
        }
    }
}

impl Drop for TestProcess {
    fn drop(&mut self) {
        let _ = self.identity.signal(libc::SIGKILL);
        let _ = self.child.wait();
    }
}

#[cfg(target_os = "macos")]
fn required_environment_helper_path() -> PathBuf {
    std::env::var_os(ENVIRONMENT_HELPER_READY_ENV)
        .map(PathBuf::from)
        .unwrap()
}

#[cfg(target_os = "macos")]
fn wait_for_environment_helper(path: &Path) {
    let deadline = Instant::now() + ENVIRONMENT_HELPER_TIMEOUT;
    while !path.exists() {
        assert!(
            Instant::now() < deadline,
            "{ENVIRONMENT_HELPER_TIMEOUT_DETAIL}"
        );
        std::thread::sleep(ENVIRONMENT_HELPER_POLL);
    }
}

#[cfg(target_os = "macos")]
pub(super) fn entry(identity: ProcessIdentity, parent_pid: u32, session_id: u32) -> InventoryEntry {
    let metadata = super::macos_native::kernel_process_metadata(identity.pid).unwrap();
    let parent_unique_id = (parent_pid > 0)
        .then(|| {
            super::macos_native::kernel_process_metadata(parent_pid)
                .unwrap()
                .unique_id
        })
        .unwrap_or_default();
    InventoryEntry {
        pid: identity.pid,
        parent_pid,
        process_group: identity.pid,
        session_id,
        start_time: identity.start_time,
        unique_id: metadata.unique_id,
        parent_unique_id,
    }
}

#[cfg(feature = "yolo-integration-tests")]
#[path = "../../integration_tests/inventory/inventory_test_support_tests.rs"]
mod tests;

#[cfg(not(target_os = "macos"))]
pub(super) fn entry(identity: ProcessIdentity, parent_pid: u32, session_id: u32) -> InventoryEntry {
    InventoryEntry {
        pid: identity.pid,
        parent_pid,
        process_group: identity.pid,
        session_id,
        start_time: identity.start_time,
        #[cfg(target_os = "linux")]
        effective_user_id: unsafe { libc::geteuid() },
    }
}
