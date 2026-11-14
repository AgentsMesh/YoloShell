use crate::process::{self, ProcessIdentity, ReservedDetachedChildSettlement};
use crate::runtime_constants::{TEST_HARNESS_EXACT_ARGUMENT, TEST_HARNESS_NOCAPTURE_ARGUMENT};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus};
use std::time::{Duration, Instant};
const HELPER_TEST: &str = "macos_secondary_custody::settlement_tests::inherited_writer_tree_helper";
const ROLE_ENV: &str = "YOLO_SECONDARY_CUSTODY_TREE_ROLE";
const LEAF_PID_ENV: &str = "YOLO_SECONDARY_CUSTODY_TREE_LEAF_PID";
const READY_ENV: &str = "YOLO_SECONDARY_CUSTODY_TREE_READY";
const ROOT_ROLE: &str = "root";
const INTERMEDIATE_ROLE: &str = "intermediate";
const LEAF_ROLE: &str = "leaf";
const LEAF_PID_FILE: &str = "leaf.pid";
const READY_FILE: &str = "tree.ready";
const EMPTY_MARKER: &[u8] = b"";
const TREE_TIMEOUT: Duration = Duration::from_secs(5);
const TREE_POLL_INTERVAL: Duration = Duration::from_millis(10);
const TREE_TIMEOUT_DETAIL: &str = "inherited writer tree convergence timed out";
const SETSID_FAILURE: libc::pid_t = -1;
pub(super) struct InheritedWriterTree {
    _directory: tempfile::TempDir,
    root: Option<(Child, ReservedDetachedChildSettlement)>,
    pub(super) root_identity: ProcessIdentity,
    leaf_pid_path: PathBuf,
    ready_path: PathBuf,
    leaf: Option<ProcessIdentity>,
}

impl InheritedWriterTree {
    pub(super) fn spawn(writer_descriptor: libc::c_int, descriptor_flags: libc::c_int) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let leaf_pid_path = directory.path().join(LEAF_PID_FILE);
        let ready_path = directory.path().join(READY_FILE);
        let mut command = helper_command(ROOT_ROLE, &leaf_pid_path, &ready_path);
        let root_settlement = process::reserve_detached_child_settlement().unwrap();
        unsafe {
            command.pre_exec(move || {
                super::set_descriptor_flags_io(
                    writer_descriptor,
                    descriptor_flags & !libc::FD_CLOEXEC,
                )
            });
        }
        let root = crate::process::spawn_waitable_test_child(command);
        let root_identity = ProcessIdentity::capture(root.id()).unwrap();
        Self {
            _directory: directory,
            root: Some((root, root_settlement)),
            root_identity,
            leaf_pid_path,
            ready_path,
            leaf: None,
        }
    }

    pub(super) fn wait_for_leaf(&mut self) -> ProcessIdentity {
        wait_for(&self.ready_path, |path| path.exists().then_some(()));
        let leaf = wait_for(&self.leaf_pid_path, read_identity);
        self.leaf = Some(leaf);
        leaf
    }

    pub(super) fn terminate_root_preserving_leaf(&mut self) {
        assert!(self.root_identity.signal(libc::SIGKILL).unwrap());
        let status = wait_for_child(&mut self.root);
        assert!(!status.success());
    }

    pub(super) fn assert_tree_terminated(&mut self) {
        if self.root.is_some() {
            let status = wait_for_child(&mut self.root);
            assert!(!status.success());
        }
        let leaf = self.leaf.unwrap();
        wait_for_identity_exit(leaf);
    }
}
impl Drop for InheritedWriterTree {
    fn drop(&mut self) {
        let leaf = self.leaf.or_else(|| read_identity(&self.leaf_pid_path));
        if let Some(leaf) = leaf {
            let _ = leaf.signal(libc::SIGKILL);
        }
        if let Some((root, settlement)) = self.root.take() {
            settlement.terminate(root);
            let deadline = Instant::now() + TREE_TIMEOUT;
            let _ = process::wait_for_detached_termination_delivery(deadline);
        }
    }
}

pub(super) fn run_role() {
    match std::env::var(ROLE_ENV).as_deref() {
        Ok(ROOT_ROLE) => run_root(),
        Ok(INTERMEDIATE_ROLE) => run_intermediate(),
        Ok(LEAF_ROLE) => run_leaf(),
        _ => {}
    }
}
fn run_root() {
    ignore_soft_termination();
    let mut intermediate = spawn_helper(INTERMEDIATE_ROLE);
    assert!(intermediate.wait().unwrap().success());
    std::fs::write(required_path(READY_ENV), EMPTY_MARKER).unwrap();
    park_forever();
}
fn run_intermediate() {
    let leaf = spawn_helper(LEAF_ROLE);
    std::mem::forget(leaf);
    unsafe { libc::_exit(libc::EXIT_SUCCESS) };
}

fn run_leaf() {
    assert_ne!(unsafe { libc::setsid() }, SETSID_FAILURE);
    ignore_soft_termination();
    std::fs::write(required_path(LEAF_PID_ENV), std::process::id().to_string()).unwrap();
    park_forever();
}

fn spawn_helper(role: &str) -> Child {
    crate::process::spawn_waitable_test_child(
        Command::new(std::env::current_exe().unwrap())
            .args([
                TEST_HARNESS_EXACT_ARGUMENT,
                HELPER_TEST,
                TEST_HARNESS_NOCAPTURE_ARGUMENT,
            ])
            .env(ROLE_ENV, role),
    )
}

fn helper_command(role: &str, leaf_pid: &Path, ready: &Path) -> Command {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            TEST_HARNESS_EXACT_ARGUMENT,
            HELPER_TEST,
            TEST_HARNESS_NOCAPTURE_ARGUMENT,
        ])
        .env(ROLE_ENV, role)
        .env(LEAF_PID_ENV, leaf_pid)
        .env(READY_ENV, ready);
    command
}

fn required_path(name: &str) -> PathBuf {
    std::env::var_os(name).map(PathBuf::from).unwrap()
}

fn read_identity(path: &Path) -> Option<ProcessIdentity> {
    let pid = std::fs::read_to_string(path).ok()?.trim().parse().ok()?;
    ProcessIdentity::capture(pid).ok()
}

fn wait_for<T>(path: &Path, inspect: impl Fn(&Path) -> Option<T>) -> T {
    let deadline = Instant::now() + TREE_TIMEOUT;
    loop {
        if let Some(value) = inspect(path) {
            return value;
        }
        assert!(deadline > Instant::now(), "{TREE_TIMEOUT_DETAIL}");
        std::thread::sleep(TREE_POLL_INTERVAL);
    }
}

fn wait_for_child(child: &mut Option<(Child, ReservedDetachedChildSettlement)>) -> ExitStatus {
    let deadline = Instant::now() + TREE_TIMEOUT;
    loop {
        if let Some(status) = child.as_mut().unwrap().0.try_wait().unwrap() {
            child.take();
            return status;
        }
        assert!(Instant::now() < deadline, "{TREE_TIMEOUT_DETAIL}");
        std::thread::sleep(TREE_POLL_INTERVAL);
    }
}

fn wait_for_identity_exit(identity: ProcessIdentity) {
    let deadline = Instant::now() + TREE_TIMEOUT;
    while identity.matches() {
        assert!(Instant::now() < deadline, "{TREE_TIMEOUT_DETAIL}");
        std::thread::sleep(TREE_POLL_INTERVAL);
    }
}

fn ignore_soft_termination() {
    unsafe {
        libc::signal(libc::SIGHUP, libc::SIG_IGN);
        libc::signal(libc::SIGTERM, libc::SIG_IGN);
    }
}

fn park_forever() -> ! {
    loop {
        std::thread::park();
    }
}
