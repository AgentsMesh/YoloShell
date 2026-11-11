use super::signal_hook::install_pre_signal_hook;
use super::{CapturedProcessTree, DetachedStandardError, ProcessIdentity, spawn_detached};
use std::cell::RefCell;
use std::mem::MaybeUninit;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant};

const SHELL_EXECUTABLE: &str = "/bin/sh";
const SHELL_COMMAND_ARGUMENT: &str = "-c";
const SCRIPT_NAME: &str = "captured-tree-race";
const FORK_SCRIPT: &str =
    "while [ ! -f \"$1\" ]; do /bin/sleep \"$3\"; done; /bin/sleep \"$4\" & echo $! > \"$2\"; wait";
const TRIGGER_FILE_NAME: &str = "fork.trigger";
const PID_FILE_NAME: &str = "late-child.pid";
const SHELL_POLL_SECONDS: &str = "0.01";
const CHILD_LIFETIME_SECONDS: &str = "30";
const WAITID_OPTIONS: i32 = libc::WEXITED | libc::WNOWAIT | libc::WNOHANG;
const NO_EXIT_PID: libc::pid_t = 0;
const TEST_TIMEOUT: Duration = Duration::from_secs(5);
const TEST_POLL_INTERVAL: Duration = Duration::from_millis(20);
const LATE_CHILD_MISSING_DETAIL: &str = "late session child was not created";
const LATE_CHILD_SURVIVED_DETAIL: &str = "late session child survived captured cleanup";

struct RaceFixture {
    _directory: tempfile::TempDir,
    child: Option<std::process::Child>,
    tree: CapturedProcessTree,
    late: Rc<RefCell<Option<ProcessIdentity>>>,
}

impl RaceFixture {
    fn spawn() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let trigger = directory.path().join(TRIGGER_FILE_NAME);
        let pid_file = directory.path().join(PID_FILE_NAME);
        let mut child = spawn_shell(&trigger, &pid_file);
        let root = ProcessIdentity::capture(child.id()).unwrap();
        let tree = CapturedProcessTree::capture(root).unwrap_or_else(|error| {
            let _ = root.signal(libc::SIGKILL);
            let _ = child.wait();
            panic!("capture dynamic test tree: {error}");
        });
        Self {
            _directory: directory,
            child: Some(child),
            tree,
            late: Rc::new(RefCell::new(None)),
        }
    }

    fn install_fork_hook(&self) -> super::signal_hook::TestSignalHookGuard {
        let trigger = self._directory.path().join(TRIGGER_FILE_NAME);
        let pid_file = self._directory.path().join(PID_FILE_NAME);
        let late = Rc::clone(&self.late);
        install_pre_signal_hook(move || {
            std::fs::write(trigger, []).unwrap();
            *late.borrow_mut() = Some(wait_for_identity(&pid_file));
        })
    }

    fn reap_root(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.wait();
        }
    }
}

impl Drop for RaceFixture {
    fn drop(&mut self) {
        let _ = self.tree.terminate(libc::SIGKILL);
        if let Some(late) = *self.late.borrow() {
            let _ = late.signal(libc::SIGKILL);
        }
        let _ = self.tree.root().signal(libc::SIGKILL);
        self.reap_root();
    }
}

#[test]
fn refresh_after_root_sigkill_captures_a_child_forked_after_initial_refresh() {
    let mut fixture = RaceFixture::spawn();
    let hook = fixture.install_fork_hook();

    fixture.tree.terminate(libc::SIGKILL).unwrap();
    drop(hook);
    wait_for_root_exit_unreaped(fixture.tree.root());
    let proof = fixture.tree.root_exit_proof_for_test(fixture.tree.root());
    fixture.tree.mark_root_exited_unreaped(proof).unwrap();
    let late = (*fixture.late.borrow()).expect(LATE_CHILD_MISSING_DETAIL);
    assert!(late.matches());

    let refreshed = fixture.tree.refresh().unwrap();
    assert!(refreshed.contains(&late));
    fixture.tree.terminate(libc::SIGKILL).unwrap();
    wait_for_identity_gone(late);
    fixture.tree.retire_session_for_test();
    fixture.reap_root();
}

fn spawn_shell(trigger: &Path, pid_file: &Path) -> std::process::Child {
    spawn_detached(
        Path::new(SHELL_EXECUTABLE),
        &[
            SHELL_COMMAND_ARGUMENT.into(),
            FORK_SCRIPT.into(),
            SCRIPT_NAME.into(),
            trigger.into(),
            pid_file.into(),
            SHELL_POLL_SECONDS.into(),
            CHILD_LIFETIME_SECONDS.into(),
        ],
        DetachedStandardError::Null,
    )
    .unwrap()
}

fn wait_for_identity(path: &PathBuf) -> ProcessIdentity {
    let deadline = Instant::now() + TEST_TIMEOUT;
    loop {
        if let Ok(value) = std::fs::read_to_string(path)
            && let Ok(pid) = value.trim().parse::<u32>()
            && let Ok(identity) = ProcessIdentity::capture(pid)
        {
            return identity;
        }
        assert!(Instant::now() < deadline, "{LATE_CHILD_MISSING_DETAIL}");
        std::thread::sleep(TEST_POLL_INTERVAL);
    }
}

fn wait_for_identity_gone(identity: ProcessIdentity) {
    let deadline = Instant::now() + TEST_TIMEOUT;
    while identity.matches() {
        assert!(Instant::now() < deadline, "{LATE_CHILD_SURVIVED_DETAIL}");
        std::thread::sleep(TEST_POLL_INTERVAL);
    }
}

fn wait_for_root_exit_unreaped(root: ProcessIdentity) {
    let deadline = Instant::now() + TEST_TIMEOUT;
    loop {
        let mut information = MaybeUninit::<libc::siginfo_t>::zeroed();
        let result = unsafe {
            libc::waitid(
                libc::P_PID,
                root.pid as libc::id_t,
                information.as_mut_ptr(),
                WAITID_OPTIONS,
            )
        };
        assert_eq!(
            result,
            0,
            "waitid failed: {}",
            std::io::Error::last_os_error()
        );
        if unsafe { information.assume_init().si_pid() } != NO_EXIT_PID {
            return;
        }
        assert!(Instant::now() < deadline, "root exit was not observed");
        std::thread::sleep(TEST_POLL_INTERVAL);
    }
}
