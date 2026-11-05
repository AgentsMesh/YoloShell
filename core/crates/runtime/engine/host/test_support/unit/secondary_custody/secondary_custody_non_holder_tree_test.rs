use crate::constants::{TEST_HARNESS_EXACT_ARGUMENT, TEST_HARNESS_NOCAPTURE_ARGUMENT};
use crate::process::{ProcessIdentity, ProcessIdentityObservation};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::Instant;

const HELPER_TEST: &str = "host::secondary_custody::tests::failure_tests::liveness_adversarial_tests::escaped_non_holder_tree_helper";
const ROLE_ENV: &str = "YOLO_HOST_CUSTODY_NON_HOLDER_ROLE";
const LEAF_PID_ENV: &str = "YOLO_HOST_CUSTODY_NON_HOLDER_LEAF_PID";
const ROOT_ROLE: &str = "root";
const LEAF_ROLE: &str = "leaf";
const LEAF_PID_FILE: &str = "leaf.pid";
const TREE_TIMEOUT: std::time::Duration = crate::session_timing::WORKER_READY_TIMEOUT;
const TREE_POLL_INTERVAL: std::time::Duration = crate::host::PROCESS_EXIT_POLL_INTERVAL;
const TREE_TIMEOUT_DETAIL: &str = "escaped non-holder process tree did not converge";

pub(super) struct EscapedNonHolderTree {
    _directory: tempfile::TempDir,
    root: Option<Child>,
    root_identity: ProcessIdentity,
    leaf_pid_path: PathBuf,
    leaf: Option<ProcessIdentity>,
}

impl EscapedNonHolderTree {
    pub(super) fn spawn() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let leaf_pid_path = directory.path().join(LEAF_PID_FILE);
        let root = spawn_helper(ROOT_ROLE, &leaf_pid_path);
        let root_identity = ProcessIdentity::capture(root.id()).unwrap();
        Self {
            _directory: directory,
            root: Some(root),
            root_identity,
            leaf_pid_path,
            leaf: None,
        }
    }

    pub(super) fn root_identity(&self) -> ProcessIdentity {
        self.root_identity
    }

    pub(super) fn wait_for_leaf(&mut self) -> ProcessIdentity {
        let leaf = wait_for_leaf(&self.leaf_pid_path);
        self.leaf = Some(leaf);
        leaf
    }
}

impl Drop for EscapedNonHolderTree {
    fn drop(&mut self) {
        let leaf = self.leaf.or_else(|| read_identity(&self.leaf_pid_path));
        if let Some(leaf) = leaf {
            let _ = leaf.signal(libc::SIGKILL);
        }
        if let Some(mut root) = self.root.take() {
            let _ = root.kill();
            let _ = root.wait();
        }
        if let Some(leaf) = leaf {
            let _ = wait_for_exit(leaf);
        }
    }
}

pub(super) fn run_role() {
    match std::env::var(ROLE_ENV).as_deref() {
        Ok(ROOT_ROLE) => run_root(),
        Ok(LEAF_ROLE) => run_leaf(),
        _ => {}
    }
}

fn run_root() {
    let leaf_pid_path = required_path(LEAF_PID_ENV);
    let leaf = spawn_helper(LEAF_ROLE, &leaf_pid_path);
    std::mem::forget(leaf);
    park_forever();
}

fn run_leaf() {
    let leaf_pid_path = required_path(LEAF_PID_ENV);
    std::fs::write(leaf_pid_path, std::process::id().to_string()).unwrap();
    park_forever();
}

fn spawn_helper(role: &str, leaf_pid_path: &Path) -> Child {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            TEST_HARNESS_EXACT_ARGUMENT,
            HELPER_TEST,
            TEST_HARNESS_NOCAPTURE_ARGUMENT,
        ])
        .env(ROLE_ENV, role)
        .env(LEAF_PID_ENV, leaf_pid_path);
    crate::process::spawn_detached_with_control_pipes(command).unwrap()
}

fn required_path(name: &str) -> PathBuf {
    std::env::var_os(name).map(PathBuf::from).unwrap()
}

fn wait_for_leaf(path: &Path) -> ProcessIdentity {
    let deadline = Instant::now() + TREE_TIMEOUT;
    loop {
        if let Some(identity) = read_identity(path) {
            return identity;
        }
        assert!(Instant::now() < deadline, "{TREE_TIMEOUT_DETAIL}");
        std::thread::sleep(TREE_POLL_INTERVAL);
    }
}

fn read_identity(path: &Path) -> Option<ProcessIdentity> {
    let pid = std::fs::read_to_string(path).ok()?.trim().parse().ok()?;
    ProcessIdentity::capture(pid).ok()
}

fn wait_for_exit(identity: ProcessIdentity) -> bool {
    let deadline = Instant::now() + TREE_TIMEOUT;
    while identity.observe() == ProcessIdentityObservation::Matching && Instant::now() < deadline {
        std::thread::sleep(TREE_POLL_INTERVAL);
    }
    identity.observe() == ProcessIdentityObservation::Gone
}

fn park_forever() -> ! {
    loop {
        std::thread::park();
    }
}
