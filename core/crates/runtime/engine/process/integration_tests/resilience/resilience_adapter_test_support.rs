use super::live_identity::ProcessExecutionState;
use super::{
    DetachedStandardError, LiveProcessIdentity, ProcessIdentity, ResilienceFreezePoll,
    ResilienceFreezingProcessTree, ResilienceFrozenProcessTree, ResilienceProcessTree,
    spawn_detached,
};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Child;
use std::time::{Duration, Instant};

const SHELL_EXECUTABLE: &str = "/bin/sh";
const SHELL_COMMAND_ARGUMENT: &str = "-c";
const SHELL_SCRIPT_NAME: &str = "resilience-adapter-tree";
const TREE_SCRIPT: &str = "/bin/sleep \"$2\" & first=$!; /bin/sleep \"$2\" & second=$!; printf '%s\\n%s\\n' \"$first\" \"$second\" > \"$1\"; wait";
const SLEEP_SECONDS: &str = "30";
const PID_FILE_NAME: &str = "descendants.pid";
const EXPECTED_DESCENDANTS: usize = 2;
const EXPECTED_TREE_TARGETS: usize = EXPECTED_DESCENDANTS + 1;
const FAILURE_DESCENDANT_INDEX: usize = 1;
const TEST_TIMEOUT: Duration = Duration::from_secs(5);
const TEST_POLL_INTERVAL: Duration = Duration::from_millis(10);
const PID_DISCOVERY_TIMEOUT_DETAIL: &str = "adapter test descendants were not ready";
const TREE_CAPTURE_DETAIL: &str = "adapter capture did not retain the complete process tree";
const PROCESS_STATE_TIMEOUT_DETAIL: &str = "adapter target did not reach the expected state";
const PROCESS_EXIT_TIMEOUT_DETAIL: &str = "adapter target did not exit";
const FREEZE_TIMEOUT_DETAIL: &str = "adapter process tree did not freeze";

pub(super) struct AdapterTreeFixture {
    _directory: tempfile::TempDir,
    root: Option<Child>,
    identities: Vec<ProcessIdentity>,
    failure_target: ProcessIdentity,
}

impl AdapterTreeFixture {
    pub(super) fn spawn() -> (Self, ResilienceProcessTree) {
        let directory = tempfile::tempdir().unwrap();
        let pid_file = directory.path().join(PID_FILE_NAME);
        let mut root = spawn_root(&pid_file);
        let descendants = wait_for_descendants(&pid_file);
        assert!(root.try_wait().unwrap().is_none(), "{TREE_CAPTURE_DETAIL}");
        let root_identity = ProcessIdentity::capture(root.id()).unwrap();
        let tree = ResilienceProcessTree::capture(root_identity).unwrap_or_else(|error| {
            let _ = root_identity.signal(libc::SIGKILL);
            let _ = root.wait();
            panic!("{TREE_CAPTURE_DETAIL}: {error:#}");
        });
        let identities = tree.identities();
        assert_eq!(
            identities.len(),
            EXPECTED_TREE_TARGETS,
            "{TREE_CAPTURE_DETAIL}"
        );
        assert!(identities.contains(&root_identity), "{TREE_CAPTURE_DETAIL}");
        assert!(
            descendants.iter().all(|target| identities.contains(target)),
            "{TREE_CAPTURE_DETAIL}"
        );
        let failure_target = descendants[FAILURE_DESCENDANT_INDEX];
        (
            Self {
                _directory: directory,
                root: Some(root),
                identities,
                failure_target,
            },
            tree,
        )
    }

    pub(super) fn identities(&self) -> &[ProcessIdentity] {
        &self.identities
    }

    pub(super) fn failure_target(&self) -> ProcessIdentity {
        self.failure_target
    }

    pub(super) fn failure_target_index(&self) -> usize {
        self.identities
            .iter()
            .position(|identity| *identity == self.failure_target)
            .expect(TREE_CAPTURE_DETAIL)
    }

    pub(super) fn reap_root(&mut self) {
        assert!(self.try_reap_root(), "{PROCESS_EXIT_TIMEOUT_DETAIL}");
    }

    fn try_reap_root(&mut self) -> bool {
        let deadline = Instant::now() + TEST_TIMEOUT;
        while let Some(root) = self.root.as_mut() {
            match root.try_wait() {
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
            std::thread::sleep(TEST_POLL_INTERVAL);
        }
        true
    }
}

impl Drop for AdapterTreeFixture {
    fn drop(&mut self) {
        for identity in &self.identities {
            let _ = identity.signal(libc::SIGCONT);
            let _ = identity.signal(libc::SIGKILL);
        }
        let _ = self.try_reap_root();
    }
}

pub(super) fn freeze(mut freezing: ResilienceFreezingProcessTree) -> ResilienceFrozenProcessTree {
    let deadline = Instant::now() + TEST_TIMEOUT;
    loop {
        freezing = match freezing.poll() {
            Ok(ResilienceFreezePoll::Pending(next)) => next,
            Ok(ResilienceFreezePoll::Frozen(frozen)) => return frozen,
            Err(error) => panic!("{FREEZE_TIMEOUT_DETAIL}: {error:?}"),
        };
        assert!(Instant::now() < deadline, "{FREEZE_TIMEOUT_DETAIL}");
        std::thread::sleep(TEST_POLL_INTERVAL);
    }
}

pub(super) fn wait_running(identities: &[ProcessIdentity]) {
    wait_for_state(identities, ProcessExecutionState::Running);
}

pub(super) fn wait_stopped(identities: &[ProcessIdentity]) {
    wait_for_state(identities, ProcessExecutionState::Stopped);
}

pub(super) fn wait_gone(identities: &[ProcessIdentity]) {
    let deadline = Instant::now() + TEST_TIMEOUT;
    while identities.iter().any(|identity| identity.matches()) {
        assert!(Instant::now() < deadline, "{PROCESS_EXIT_TIMEOUT_DETAIL}");
        std::thread::sleep(TEST_POLL_INTERVAL);
    }
}

fn wait_for_state(identities: &[ProcessIdentity], expected: ProcessExecutionState) {
    let deadline = Instant::now() + TEST_TIMEOUT;
    loop {
        let reached = identities.iter().all(|identity| {
            LiveProcessIdentity::capture(*identity)
                .unwrap()
                .is_some_and(|live| live.execution_state().ok().as_ref() == Some(&expected))
        });
        if reached {
            return;
        }
        assert!(Instant::now() < deadline, "{PROCESS_STATE_TIMEOUT_DETAIL}");
        std::thread::sleep(TEST_POLL_INTERVAL);
    }
}

fn spawn_root(pid_file: &Path) -> Child {
    let arguments = [
        OsString::from(SHELL_COMMAND_ARGUMENT),
        OsString::from(TREE_SCRIPT),
        OsString::from(SHELL_SCRIPT_NAME),
        pid_file.as_os_str().to_owned(),
        OsString::from(SLEEP_SECONDS),
    ];
    spawn_detached(
        Path::new(SHELL_EXECUTABLE),
        &arguments,
        DetachedStandardError::Null,
    )
    .unwrap()
}

fn wait_for_descendants(pid_file: &PathBuf) -> Vec<ProcessIdentity> {
    let deadline = Instant::now() + TEST_TIMEOUT;
    loop {
        if let Ok(contents) = std::fs::read_to_string(pid_file) {
            let descendants = contents
                .lines()
                .filter_map(|value| value.parse::<u32>().ok())
                .filter_map(|pid| ProcessIdentity::capture(pid).ok())
                .collect::<Vec<_>>();
            if descendants.len() == EXPECTED_DESCENDANTS {
                return descendants;
            }
        }
        assert!(Instant::now() < deadline, "{PID_DISCOVERY_TIMEOUT_DETAIL}");
        std::thread::sleep(TEST_POLL_INTERVAL);
    }
}
