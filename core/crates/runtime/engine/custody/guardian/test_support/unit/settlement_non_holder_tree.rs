use crate::constants::{TEST_HARNESS_EXACT_ARGUMENT, TEST_HARNESS_NOCAPTURE_ARGUMENT};
use crate::custody_guardian::test_child_guard::GuardedChild;
use crate::process::{ProcessIdentity, ProcessIdentityObservation};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::Instant;

const HELPER_TEST: &str = "custody_guardian::settlement_containment::adversarial_tests::escaped_non_holder_descendant_helper";
const ROLE_ENV: &str = "YOLO_GUARDIAN_NON_HOLDER_ROLE";
const LEAF_PID_ENV: &str = "YOLO_GUARDIAN_NON_HOLDER_LEAF_PID";
const ROOT_ROLE: &str = "root";
const LEAF_ROLE: &str = "leaf";
const LEAF_PID_FILE: &str = "leaf.pid";
const TREE_TIMEOUT: std::time::Duration = crate::session_timing::WORKER_READY_TIMEOUT;
const TREE_POLL_INTERVAL: std::time::Duration =
    crate::session_timing::GRACEFUL_TERMINATION_POLL_INTERVAL;
const TREE_TIMEOUT_DETAIL: &str = "Guardian non-holder process tree did not converge";

pub(super) struct NonHolderTree {
    _directory: tempfile::TempDir,
    root: GuardedChild,
    root_identity: ProcessIdentity,
    leaf_pid_path: PathBuf,
    leaf: Option<ProcessIdentity>,
}

impl NonHolderTree {
    pub(super) fn spawn() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let leaf_pid_path = directory.path().join(LEAF_PID_FILE);
        let root = GuardedChild::spawn(|| spawn_helper(ROOT_ROLE, &leaf_pid_path));
        let root_identity = root.identity();
        Self {
            _directory: directory,
            root,
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

    pub(super) fn wait_for_root_exit(&mut self) -> std::process::ExitStatus {
        let deadline = Instant::now() + TREE_TIMEOUT;
        loop {
            if let Some(status) = self.root.try_wait().unwrap() {
                return status;
            }
            assert!(Instant::now() < deadline, "{TREE_TIMEOUT_DETAIL}");
            std::thread::sleep(TREE_POLL_INTERVAL);
        }
    }

    pub(super) fn wait_for_leaf_exit(&self) -> bool {
        self.leaf.is_some_and(wait_for_exit)
    }
}

impl Drop for NonHolderTree {
    fn drop(&mut self) {
        let leaf = self.leaf.or_else(|| read_identity(&self.leaf_pid_path));
        if let Some(leaf) = leaf {
            let _ = leaf.enforce_exact_test_cleanup();
        }
        let _ = self.root.request_cleanup();
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
    ignore_shutdown_signals();
    let leaf_pid_path = required_path(LEAF_PID_ENV);
    let leaf = spawn_helper(LEAF_ROLE, &leaf_pid_path);
    std::mem::forget(leaf);
    park_forever();
}

fn run_leaf() {
    ignore_shutdown_signals();
    let leaf_pid_path = required_path(LEAF_PID_ENV);
    std::fs::write(leaf_pid_path, std::process::id().to_string()).unwrap();
    park_forever();
}

fn ignore_shutdown_signals() {
    unsafe {
        libc::signal(libc::SIGHUP, libc::SIG_IGN);
        libc::signal(libc::SIGTERM, libc::SIG_IGN);
    }
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
