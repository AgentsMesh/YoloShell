use crate::process::{
    PROCESS_DISCOVERY_CAPABILITY_ENV, ProcessDiscoveryCapability, ProcessIdentity,
};
use std::path::{Path, PathBuf};
use std::process::{Child, Stdio};
use std::time::{Duration, Instant};
#[path = "captured_tree_discovery_helper_macos.rs"]
mod helper;

const ROOT_READY_FILE: &str = "root.ready";
const TRIGGER_FILE: &str = "trigger";
const LEAF_PID_FILE: &str = "leaf.pid";
const LEAF_READY_FILE: &str = "leaf.ready";
const EMPTY_MARKER: &[u8] = b"";
const CLEANUP_TIMEOUT: Duration = helper::WAIT_TIMEOUT;
const POLL_INTERVAL: Duration = Duration::from_millis(10);
const HELPER_TIMEOUT_DETAIL: &str = "rapid escaped descendant helper timed out";
const HELPER_EARLY_EXIT_DETAIL: &str = "rapid escaped descendant helper exited early";
pub(super) struct RapidEscapeFixture {
    _directory: tempfile::TempDir,
    root: Option<Child>,
    root_identity: ProcessIdentity,
    trigger: PathBuf,
    leaf_pid: PathBuf,
    leaf_ready: PathBuf,
    leaf: Option<ProcessIdentity>,
}
impl RapidEscapeFixture {
    pub(super) fn spawn(capability: &str) -> Self {
        Self::spawn_configured(capability, |_command| {}, None)
    }

    pub(super) fn spawn_with_scrubbed_leaf_capability(
        capability: &ProcessDiscoveryCapability,
    ) -> Self {
        Self::spawn_with_capability_configuration(capability, |command| {
            helper::configure_leaf_environment_scrub(command);
        })
    }

    fn spawn_with_capability_configuration(
        capability: &ProcessDiscoveryCapability,
        configure: impl FnOnce(&mut std::process::Command),
    ) -> Self {
        let inheritance = capability.begin_writer_inheritance().unwrap();
        let boundary = inheritance.spawn_boundary();
        let fixture = Self::spawn_configured(
            capability.value(),
            |command| {
                inheritance.configure_child(capability, command).unwrap();
                configure(command);
            },
            Some(&boundary),
        );
        capability
            .commit_writer_inheritance(inheritance, fixture.root_identity)
            .unwrap();
        fixture
    }

    fn spawn_configured(
        capability: &str,
        configure: impl FnOnce(&mut std::process::Command),
        boundary: Option<&crate::process::WaitableChildSpawnBoundary<'_>>,
    ) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let root_ready = directory.path().join(ROOT_READY_FILE);
        let trigger = directory.path().join(TRIGGER_FILE);
        let leaf_pid = directory.path().join(LEAF_PID_FILE);
        let leaf_ready = directory.path().join(LEAF_READY_FILE);
        let mut command = helper::command(
            helper::ROOT_ROLE,
            &root_ready,
            &trigger,
            &leaf_pid,
            &leaf_ready,
        );
        command
            .env(PROCESS_DISCOVERY_CAPABILITY_ENV, capability)
            .stderr(Stdio::inherit());
        configure(&mut command);
        let mut root = match boundary {
            Some(boundary) => {
                crate::process::spawn_waitable_test_child_in_boundary(command, boundary)
            }
            None => crate::process::spawn_waitable_test_child(command),
        };
        let root_identity = ProcessIdentity::capture(root.id()).unwrap();
        wait_for_child_path(&mut root, &root_ready, helper::WAIT_TIMEOUT);
        Self {
            _directory: directory,
            root: Some(root),
            root_identity,
            trigger,
            leaf_pid,
            leaf_ready,
            leaf: None,
        }
    }

    pub(super) fn root_identity(&self) -> ProcessIdentity {
        self.root_identity
    }

    pub(super) fn release_and_capture_leaf(&mut self) -> ProcessIdentity {
        std::fs::write(&self.trigger, EMPTY_MARKER).unwrap();
        wait_for_path(&self.leaf_ready, helper::WAIT_TIMEOUT);
        let leaf = capture_pid(&self.leaf_pid);
        self.leaf = Some(leaf);
        assert!(
            self.reap_root(helper::WAIT_TIMEOUT),
            "{HELPER_TIMEOUT_DETAIL}"
        );
        leaf
    }

    pub(super) fn wait_for_leaf_exit(&self) {
        assert!(self.leaf_exited(CLEANUP_TIMEOUT), "{HELPER_TIMEOUT_DETAIL}");
    }

    fn reap_root(&mut self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        while let Some(child) = self.root.as_mut() {
            match child.try_wait() {
                Ok(Some(_)) => {
                    self.root.take();
                    return true;
                }
                Err(_) => return false,
                Ok(None) => {}
            }
            if Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(POLL_INTERVAL);
        }
        true
    }

    fn leaf_exited(&self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        while self.leaf.is_some_and(ProcessIdentity::matches) {
            if Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(POLL_INTERVAL);
        }
        true
    }
}
impl Drop for RapidEscapeFixture {
    fn drop(&mut self) {
        if self.leaf.is_none() && self.trigger.exists() {
            let deadline = Instant::now() + CLEANUP_TIMEOUT;
            while !self.leaf_pid.exists() && Instant::now() < deadline {
                std::thread::sleep(POLL_INTERVAL);
            }
            self.leaf = try_capture_pid(&self.leaf_pid);
        }
        if let Some(leaf) = self.leaf.filter(|identity| identity.matches()) {
            let _ = leaf.signal(libc::SIGKILL);
        }
        if self.root_identity.matches() {
            let _ = self.root_identity.signal(libc::SIGKILL);
        }
        if self.root.is_some() {
            let _ = self.reap_root(CLEANUP_TIMEOUT);
        }
        let _ = self.leaf_exited(CLEANUP_TIMEOUT);
    }
}

fn wait_for_path(path: &Path, timeout: Duration) {
    let deadline = Instant::now() + timeout;
    while !path.exists() {
        assert!(Instant::now() < deadline, "{HELPER_TIMEOUT_DETAIL}");
        std::thread::sleep(POLL_INTERVAL);
    }
}

fn wait_for_child_path(child: &mut Child, path: &Path, timeout: Duration) {
    let deadline = Instant::now() + timeout;
    while !path.exists() {
        if let Some(status) = child.try_wait().unwrap() {
            panic!("{HELPER_EARLY_EXIT_DETAIL}: {status}");
        }
        assert!(Instant::now() < deadline, "{HELPER_TIMEOUT_DETAIL}");
        std::thread::sleep(POLL_INTERVAL);
    }
}

fn capture_pid(path: &Path) -> ProcessIdentity {
    try_capture_pid(path).unwrap()
}

fn try_capture_pid(path: &Path) -> Option<ProcessIdentity> {
    let pid = std::fs::read_to_string(path).ok()?.trim().parse().ok()?;
    ProcessIdentity::capture(pid).ok()
}
