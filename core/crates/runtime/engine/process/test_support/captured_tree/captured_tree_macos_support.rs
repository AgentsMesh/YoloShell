use super::captured_tree_macos_wait::{
    POLL_INTERVAL, TEST_TIMEOUT, wait_for_exit_unreaped, wait_for_path, wait_for_reparent,
};
use super::{LiveProcessIdentity, ProcessIdentity};
use crate::runtime_constants::{TEST_HARNESS_EXACT_ARGUMENT, TEST_HARNESS_NOCAPTURE_ARGUMENT};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

#[path = "captured_tree_macos_support_io.rs"]
mod support_io;
use support_io::{capture_pid_file, required_path};

#[cfg(feature = "yolo-unit-tests")]
const HELPER_TEST: &str = "captured_tree_macos_support::lineage_helper_unit";
#[cfg(feature = "yolo-integration-tests")]
const HELPER_TEST: &str = "captured_tree_macos_support::lineage_helper_integration";
#[cfg(all(not(test), feature = "runtime-component-tests"))]
const HELPER_TEST: &str = stringify!(process_lineage_fixture_entrypoint);
const ROLE_ENV: &str = "YOLO_LINEAGE_HELPER_ROLE";
const DIRECTORY_ENV: &str = "YOLO_LINEAGE_HELPER_DIRECTORY";
const ROOT_ROLE: &str = "root";
const LEAF_ROLE: &str = "leaf";
const ROOT_READY_FILE: &str = "root.ready";
const TRIGGER_FILE: &str = "trigger";
const LEAF_READY_FILE: &str = "leaf.ready";
const LEAF_PID_FILE: &str = "leaf.pid";
const EMPTY_FILE: &[u8] = b"";
const LEAF_LIFETIME: Duration = Duration::from_secs(30);
const CLEANUP_DISCOVERY_TIMEOUT: Duration = Duration::from_secs(1);
const SETSID_FAILURE: libc::pid_t = -1;
const CAPTURE_LEAF_FAILURE_DETAIL: &str = "capture lineage leaf identity";
const LEAF_NOT_CAPTURED_DETAIL: &str = "lineage leaf was not captured";
const LEAF_SURVIVED_DETAIL: &str = "lineage leaf survived cleanup";

pub struct LineageFixture {
    _directory: tempfile::TempDir,
    root: Option<Child>,
    root_identity: ProcessIdentity,
    root_live: LiveProcessIdentity,
    trigger: PathBuf,
    leaf_ready: PathBuf,
    leaf_pid: PathBuf,
    leaf: Option<(ProcessIdentity, LiveProcessIdentity)>,
}

impl LineageFixture {
    pub fn spawn() -> Self {
        Self::spawn_configured(|_command| {})
    }

    pub fn spawn_configured(configure: impl FnOnce(&mut Command)) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let root_ready = directory.path().join(ROOT_READY_FILE);
        let trigger = directory.path().join(TRIGGER_FILE);
        let leaf_ready = directory.path().join(LEAF_READY_FILE);
        let leaf_pid = directory.path().join(LEAF_PID_FILE);
        let root = spawn_helper(ROOT_ROLE, directory.path(), configure);
        wait_for_path(&root_ready);
        let root_identity = ProcessIdentity::capture(root.id()).unwrap();
        let root_live = LiveProcessIdentity::capture(root_identity)
            .unwrap()
            .unwrap();
        Self {
            _directory: directory,
            root: Some(root),
            root_identity,
            root_live,
            trigger,
            leaf_ready,
            leaf_pid,
            leaf: None,
        }
    }

    pub fn root_identity(&self) -> ProcessIdentity {
        self.root_identity
    }

    pub fn release_and_capture_leaf(&mut self) -> ProcessIdentity {
        std::fs::write(&self.trigger, EMPTY_FILE).unwrap();
        wait_for_path(&self.leaf_ready);
        let leaf = capture_pid_file(&self.leaf_pid).expect(CAPTURE_LEAF_FAILURE_DETAIL);
        self.leaf = Some(leaf);
        leaf.0
    }

    pub fn wait_for_root_exit_and_leaf_reparent(&self) {
        let leaf = self.leaf.expect(LEAF_NOT_CAPTURED_DETAIL).0;
        wait_for_exit_unreaped(self.root_identity);
        wait_for_reparent(leaf, self.root_identity.pid);
    }

    pub fn release_and_observe_leaf(&mut self) -> ProcessIdentity {
        let leaf = self.release_and_capture_leaf();
        std::fs::remove_file(&self.trigger).unwrap();
        self.wait_for_root_exit_and_leaf_reparent();
        leaf
    }

    pub fn wait_for_leaf_exit(&self) {
        let leaf = self.leaf.unwrap().1;
        let deadline = Instant::now() + TEST_TIMEOUT;
        while leaf.matches() {
            assert!(Instant::now() < deadline, "{}", LEAF_SURVIVED_DETAIL);
            std::thread::sleep(POLL_INTERVAL);
        }
    }

    pub fn reap_root(&mut self) {
        if let Some(mut root) = self.root.take() {
            root.wait().unwrap();
        }
    }
}

impl Drop for LineageFixture {
    fn drop(&mut self) {
        if self.leaf.is_none() && self.trigger.exists() {
            let deadline = Instant::now() + CLEANUP_DISCOVERY_TIMEOUT;
            while !self.leaf_pid.exists() && Instant::now() < deadline {
                std::thread::sleep(POLL_INTERVAL);
            }
            self.leaf = capture_pid_file(&self.leaf_pid);
        }
        if let Some((_durable, live)) = self.leaf {
            let _ = live.signal(libc::SIGKILL);
        }
        let _ = self.root_live.signal(libc::SIGKILL);
        self.reap_root();
    }
}

pub fn run_lineage_helper() {
    match std::env::var(ROLE_ENV).as_deref() {
        Ok(ROOT_ROLE) => run_root_helper(),
        Ok(LEAF_ROLE) => run_leaf_helper(),
        _ => {}
    }
}

#[cfg(feature = "yolo-unit-tests")]
include!("../../unit_tests/captured_tree_macos_support_entrypoint.rs");
#[cfg(feature = "yolo-integration-tests")]
include!("../../integration_tests/captured/tree/macos/captured_tree_macos_support_entrypoint.rs");

fn run_root_helper() {
    assert_ne!(unsafe { libc::setsid() }, SETSID_FAILURE);
    let directory = required_path(DIRECTORY_ENV);
    let trigger = directory.join(TRIGGER_FILE);
    std::fs::write(directory.join(ROOT_READY_FILE), EMPTY_FILE).unwrap();
    wait_for_path(&trigger);
    let _leaf = spawn_helper(LEAF_ROLE, &directory, |_command| {});
    wait_for_path(&directory.join(LEAF_READY_FILE));
    while trigger.exists() {
        std::thread::sleep(POLL_INTERVAL);
    }
}

fn run_leaf_helper() {
    assert_ne!(unsafe { libc::setsid() }, SETSID_FAILURE);
    let directory = required_path(DIRECTORY_ENV);
    unsafe {
        libc::signal(libc::SIGHUP, libc::SIG_IGN);
        libc::signal(libc::SIGTERM, libc::SIG_IGN);
    }
    std::fs::write(
        directory.join(LEAF_PID_FILE),
        std::process::id().to_string(),
    )
    .unwrap();
    std::fs::write(directory.join(LEAF_READY_FILE), EMPTY_FILE).unwrap();
    std::thread::sleep(LEAF_LIFETIME);
}

fn spawn_helper(role: &str, directory: &Path, configure: impl FnOnce(&mut Command)) -> Child {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            TEST_HARNESS_EXACT_ARGUMENT,
            HELPER_TEST,
            TEST_HARNESS_NOCAPTURE_ARGUMENT,
        ])
        .env(ROLE_ENV, role)
        .env(DIRECTORY_ENV, directory)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    configure(&mut command);
    crate::process::spawn_waitable_test_child(command)
}

#[cfg(feature = "yolo-integration-tests")]
#[path = "../../integration_tests/captured/tree/macos/captured_tree_macos_support_tests.rs"]
mod tests;
