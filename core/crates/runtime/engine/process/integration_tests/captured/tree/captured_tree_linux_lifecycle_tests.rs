use super::inventory_snapshot::install_test_inventory;
use super::inventory_test_support::{NO_PARENT_PID, TestProcess, entry};
use super::{CapturedProcessTree, DetachedStandardError, ProcessIdentity, spawn_detached};
use std::path::{Path, PathBuf};
use std::process::Child;
use std::time::{Duration, Instant};

const SHELL_EXECUTABLE: &str = "/bin/sh";
const SHELL_COMMAND_ARGUMENT: &str = "-c";
const SCRIPT_NAME: &str = "yolo-lineage-lifecycle";
const TREE_SCRIPT: &str = concat!(
    "/bin/sh -c 'while [ ! -f \"$1\" ]; do /bin/sleep \"$2\"; done' ",
    "yolo-agent \"$1\" \"$5\" & echo $! > \"$2\"; wait $!; : > \"$3\"; ",
    "exec /bin/sleep \"$4\""
);
const AGENT_RELEASE_FILE: &str = "agent.release";
const AGENT_PID_FILE: &str = "agent.pid";
const AGENT_DONE_FILE: &str = "agent.done";
const ROOT_LIFETIME_SECONDS: &str = "30";
const POLL_SECONDS: &str = "0.01";
const TEST_TIMEOUT: Duration = Duration::from_secs(5);
const TEST_POLL_INTERVAL: Duration = Duration::from_millis(20);
const AGENT_PID_DETAIL: &str = "agent pid was not published";
const AGENT_CAPTURE_DETAIL: &str = "agent was not captured";
const AGENT_EXIT_DETAIL: &str = "agent did not exit normally";

struct TreeFixture {
    _directory: tempfile::TempDir,
    root: Option<Child>,
    tree: CapturedProcessTree,
    agent: ProcessIdentity,
    release: PathBuf,
    done: PathBuf,
}

impl TreeFixture {
    fn spawn() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let release = directory.path().join(AGENT_RELEASE_FILE);
        let pid_file = directory.path().join(AGENT_PID_FILE);
        let done = directory.path().join(AGENT_DONE_FILE);
        let root = spawn_shell(&release, &pid_file, &done);
        let root_identity = ProcessIdentity::capture(root.id()).unwrap();
        let tree = CapturedProcessTree::capture(root_identity).unwrap();
        let agent = wait_for_identity(&pid_file);
        let mut fixture = Self {
            _directory: directory,
            root: Some(root),
            tree,
            agent,
            release,
            done,
        };
        wait_until(AGENT_CAPTURE_DETAIL, || {
            fixture.tree.refresh().unwrap().contains(&agent)
        });
        fixture
    }
}

impl Drop for TreeFixture {
    fn drop(&mut self) {
        let _ = self.tree.terminate(libc::SIGKILL);
        if let Some(mut root) = self.root.take() {
            let _ = root.kill();
            let _ = root.wait();
        }
    }
}

#[test]
fn shell_agent_normal_exit_interrupts_then_settles_lineage_observation() {
    let mut fixture = TreeFixture::spawn();
    std::fs::write(&fixture.release, []).unwrap();
    wait_until(AGENT_EXIT_DETAIL, || {
        fixture.done.exists() && !fixture.agent.matches()
    });

    fixture.tree.refresh().unwrap();
    assert!(!fixture.tree.descendant_observation_is_settled());
    fixture.tree.refresh().unwrap();
    assert!(fixture.tree.descendant_observation_is_settled());
    assert!(!fixture.tree.identities().contains(&fixture.agent));
}

#[test]
fn stable_candidate_lost_before_pidfd_capture_interrupts_empty_proof() {
    let root = TestProcess::spawn();
    let mut candidate = TestProcess::spawn();
    let mut tree = CapturedProcessTree::capture(root.identity).unwrap();
    let root_entry = entry(root.identity, NO_PARENT_PID, root.identity.pid);
    let candidate_entry = entry(candidate.identity, root.identity.pid, root.identity.pid);
    candidate.terminate();
    let stale_inventory = install_test_inventory([
        vec![root_entry, candidate_entry],
        vec![root_entry, candidate_entry],
    ]);

    tree.refresh().unwrap();
    assert!(!tree.descendant_observation_is_settled());
    drop(stale_inventory);
    let _stable_inventory = install_test_inventory([vec![root_entry], vec![root_entry]]);
    tree.refresh().unwrap();
    assert!(tree.descendant_observation_is_settled());
}

fn spawn_shell(release: &Path, pid_file: &Path, done: &Path) -> Child {
    spawn_detached(
        Path::new(SHELL_EXECUTABLE),
        &[
            SHELL_COMMAND_ARGUMENT.into(),
            TREE_SCRIPT.into(),
            SCRIPT_NAME.into(),
            release.into(),
            pid_file.into(),
            done.into(),
            ROOT_LIFETIME_SECONDS.into(),
            POLL_SECONDS.into(),
        ],
        DetachedStandardError::Null,
    )
    .unwrap()
}

fn wait_for_identity(path: &Path) -> ProcessIdentity {
    let mut identity = None;
    wait_until(AGENT_PID_DETAIL, || {
        identity = std::fs::read_to_string(path)
            .ok()
            .and_then(|pid| pid.trim().parse().ok())
            .and_then(|pid| ProcessIdentity::capture(pid).ok());
        identity.is_some()
    });
    identity.unwrap()
}

fn wait_until(detail: &str, mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + TEST_TIMEOUT;
    while !condition() {
        assert!(Instant::now() < deadline, "{detail}");
        std::thread::sleep(TEST_POLL_INTERVAL);
    }
}
