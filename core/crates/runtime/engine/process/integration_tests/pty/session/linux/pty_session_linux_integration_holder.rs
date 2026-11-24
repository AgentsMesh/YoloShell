use super::super::inventory_snapshot::install_test_inventory;
use super::super::inventory_test_support::{NO_PARENT_PID, entry};
use super::super::{
    CapturedProcessTree, DestructiveSessionUnavailable, PTY_SESSION_CAPABILITY_ENV, ProcessIdentity,
};
use super::support::{TestPty, configure_session, spawn_session};
use std::mem::MaybeUninit;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::{Duration, Instant};

const CLOSE_MODE: &str = "close";
const INITIAL_CAPTURE_SNAPSHOTS: usize = 4;
const HOLDER_LIFETIME_SECONDS: &str = "30";
const HOLDER_PID_FILE: &str = "holder.pid";
const POLL_SECONDS: &str = "0.01";
const RETAIN_MODE: &str = "retain";
const ROOT_RELEASE_FILE: &str = "root.release";
const REFRESH_SNAPSHOTS: usize = 2;
const SCRIPT_NAME: &str = "yolo-linux-holder-witness";
const SHELL_COMMAND_ARGUMENT: &str = "-c";
const SHELL_EXECUTABLE: &str = "/bin/sh";
const SYSTEM_CALL_FAILURE: libc::c_int = -1;
const TEST_POLL_INTERVAL: Duration = Duration::from_millis(10);
const TEST_TIMEOUT: Duration = Duration::from_secs(5);
const WAITID_OPTIONS: libc::c_int = libc::WEXITED | libc::WNOWAIT | libc::WNOHANG;
const ROOT_SCRIPT: &str = concat!(
    "/usr/bin/setsid --fork /bin/sh -c '",
    "if [ \"$3\" = close ]; then eval \"exec $4>&-\"; fi; ",
    "echo $$ > \"$1\"; exec /bin/sleep \"$2\"",
    "' yolo-holder \"$1\" \"$4\" \"$5\" \"$6\"; ",
    "while [ ! -f \"$2\" ]; do /bin/sleep \"$3\"; done"
);

#[derive(Clone, Copy)]
enum HolderMode {
    CloseSlave,
    RetainSlave,
}

impl HolderMode {
    fn argument(self) -> &'static str {
        match self {
            Self::CloseSlave => CLOSE_MODE,
            Self::RetainSlave => RETAIN_MODE,
        }
    }
}

struct HolderFixture {
    _directory: tempfile::TempDir,
    _pty: TestPty,
    root: Option<Child>,
    tree: CapturedProcessTree,
    holder: ProcessIdentity,
}

impl HolderFixture {
    fn spawn(mode: HolderMode) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let capability = directory.path().display().to_string();
        let holder_pid = directory.path().join(HOLDER_PID_FILE);
        let root_release = directory.path().join(ROOT_RELEASE_FILE);
        let mut pty = TestPty::open(&capability);
        let mut command = holder_command(
            &capability,
            &holder_pid,
            &root_release,
            mode,
            pty.slave_fd(),
        );
        let root_child = spawn_session(command);
        let root = ProcessIdentity::capture(root_child.id()).unwrap();
        pty.close_slave();
        let holder = wait_for_identity(&holder_pid);
        let root_entry = entry(root, NO_PARENT_PID, root.pid);
        let holder_entry = entry(holder, NO_PARENT_PID, holder.pid);
        let mut tree = {
            let snapshots =
                std::iter::repeat_n(vec![root_entry, holder_entry], INITIAL_CAPTURE_SNAPSHOTS);
            let _inventory = install_test_inventory(snapshots);
            CapturedProcessTree::capture_pty(root, pty.proof.clone()).unwrap()
        };
        assert!(!tree.identities().contains(&holder));
        std::fs::write(&root_release, []).unwrap();
        wait_for_unreaped(root);
        let proof = tree.root_exit_proof_for_test(root);
        tree.mark_root_exited_unreaped(proof).unwrap();
        {
            let snapshots = std::iter::repeat_n(vec![root_entry, holder_entry], REFRESH_SNAPSHOTS);
            let _inventory = install_test_inventory(snapshots);
            tree.refresh().unwrap();
        }
        Self {
            _directory: directory,
            _pty: pty,
            root: Some(root_child),
            tree,
            holder,
        }
    }

    fn request_termination(&mut self) -> anyhow::Result<Vec<ProcessIdentity>> {
        self.tree.request_graceful_descendant_termination()
    }
}

impl Drop for HolderFixture {
    fn drop(&mut self) {
        let _ = self.holder.signal(libc::SIGKILL);
        if let Some(mut root) = self.root.take() {
            let _ = root.kill();
            let _ = root.wait();
        }
    }
}

#[test]
fn orphaned_setsid_holder_is_observed_without_destructive_authority() {
    let mut fixture = HolderFixture::spawn(HolderMode::RetainSlave);

    assert!(fixture.tree.identities().contains(&fixture.holder));
    let error = fixture.request_termination().unwrap_err();
    assert!(
        error
            .downcast_ref::<DestructiveSessionUnavailable>()
            .is_some()
    );
    assert!(fixture.holder.matches());
}

#[test]
fn orphaned_setsid_process_that_closed_the_slave_is_not_owned() {
    let fixture = HolderFixture::spawn(HolderMode::CloseSlave);

    assert!(!fixture.tree.identities().contains(&fixture.holder));
    assert!(fixture.holder.matches());
}

fn holder_command(
    capability: &str,
    holder_pid: &Path,
    root_release: &Path,
    mode: HolderMode,
    slave: libc::c_int,
) -> Command {
    let mut command = Command::new(SHELL_EXECUTABLE);
    command
        .arg(SHELL_COMMAND_ARGUMENT)
        .arg(ROOT_SCRIPT)
        .arg(SCRIPT_NAME)
        .arg(holder_pid)
        .arg(root_release)
        .arg(POLL_SECONDS)
        .arg(HOLDER_LIFETIME_SECONDS)
        .arg(mode.argument())
        .arg(slave.to_string())
        .env(PTY_SESSION_CAPABILITY_ENV, capability);
    configure_session(&mut command);
    command
}

fn wait_for_identity(path: &PathBuf) -> ProcessIdentity {
    let mut identity = None;
    wait_until("setsid holder did not publish its pid", || {
        identity = std::fs::read_to_string(path)
            .ok()
            .and_then(|pid| pid.trim().parse().ok())
            .and_then(|pid| ProcessIdentity::capture(pid).ok());
        identity.is_some()
    });
    identity.unwrap()
}

fn wait_for_unreaped(root: ProcessIdentity) {
    wait_until("PTY root did not exit unreaped", || {
        let mut information = MaybeUninit::<libc::siginfo_t>::zeroed();
        let result = unsafe {
            libc::waitid(
                libc::P_PID,
                root.pid as libc::id_t,
                information.as_mut_ptr(),
                WAITID_OPTIONS,
            )
        };
        assert_ne!(result, SYSTEM_CALL_FAILURE);
        (unsafe { information.assume_init().si_pid() }) != libc::pid_t::default()
    });
}

fn wait_until(detail: &str, mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + TEST_TIMEOUT;
    while !condition() {
        assert!(Instant::now() < deadline, "{detail}");
        std::thread::sleep(TEST_POLL_INTERVAL);
    }
}
