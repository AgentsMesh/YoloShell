use super::*;
use crate::process::{CapturedProcessTree, DetachedStandardError, ProcessIdentity};
use crate::worker::pty_exit_observer::WaitidRootExitObserver;
use crate::worker::pty_ownership::{PtyOwnership, reserve_fault_reaper};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;

const CHILD_SLEEP_DURATION: &str = "30";
const CHILD_SLEEP_PREFIX: &str = "CHILD_SLEEP_SECONDS=";
const ENV_EXECUTABLE: &str = "/usr/bin/env";
const MARKER_MISSING_DETAIL: &str = "termination marker was not created";
const MARKER_POLL_INTERVAL: Duration = Duration::from_millis(5);
const MARKER_TIMEOUT: Duration = Duration::from_secs(5);
const MINIMUM_TREE_MEMBER_COUNT: usize = 2;
const PROCESS_SETTLEMENT_TIMEOUT: Duration = Duration::from_secs(10);
const READY_MARKER_NAME: &str = "ready";
const READY_MARKER_PREFIX: &str = "READY_MARKER=";
const SHELL_ARGUMENT: &str = "-c";
const SHELL_EXECUTABLE: &str = "/bin/sh";
const TERM_MARKER_NAME: &str = "term";
const TERM_MARKER_PREFIX: &str = "TERM_MARKER=";
const TERM_SCRIPT: &str = "/bin/sleep \"$CHILD_SLEEP_SECONDS\" & CHILD_PID=$!; \
    trap ': > \"$TERM_MARKER\"; kill \"$CHILD_PID\" 2>/dev/null || :; \
    wait \"$CHILD_PID\" 2>/dev/null || :; exit 0' TERM; : > \"$READY_MARKER\"; wait \"$CHILD_PID\"";
const TREE_CAPTURE_MISSING_DETAIL: &str = "captured process tree remained incomplete";
const PROCESS_SETTLEMENT_MISSING_DETAIL: &str = "test process custody did not settle";
const PROCESS_SETTLEMENT_FAULT_DETAIL: &str = "test process custody settled with a fault";

pub(super) struct GracefulFixture {
    _directory: tempfile::TempDir,
    pub(super) ownership: PtyOwnership,
    pub(super) output: PtyOutputRecovery,
    pub(super) root: ProcessIdentity,
    pub(super) term_marker: PathBuf,
}

impl GracefulFixture {
    pub(super) fn spawn() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let ready_marker = directory.path().join(READY_MARKER_NAME);
        let term_marker = directory.path().join(TERM_MARKER_NAME);
        let arguments = [
            OsString::from(format!("{CHILD_SLEEP_PREFIX}{CHILD_SLEEP_DURATION}")),
            OsString::from(format!("{READY_MARKER_PREFIX}{}", ready_marker.display())),
            OsString::from(format!("{TERM_MARKER_PREFIX}{}", term_marker.display())),
            SHELL_EXECUTABLE.into(),
            SHELL_ARGUMENT.into(),
            TERM_SCRIPT.into(),
        ];
        let child = crate::process::spawn_detached(
            Path::new(ENV_EXECUTABLE),
            &arguments,
            DetachedStandardError::Null,
        )
        .unwrap();
        wait_for_marker(&ready_marker);
        let root = ProcessIdentity::capture(child.id()).unwrap();
        let mut tree = CapturedProcessTree::capture(root).unwrap();
        wait_for_tree_members(&mut tree, MINIMUM_TREE_MEMBER_COUNT);
        tree.authorize_destructive_for_test();
        let ownership =
            PtyOwnership::new_for_test(Box::new(child), tree, reserve_fault_reaper().unwrap())
                .unwrap();
        let output = ownership.output_recovery();
        Self {
            _directory: directory,
            ownership,
            output,
            root,
            term_marker,
        }
    }

    pub(super) fn settle(mut self) {
        let deadline = Instant::now() + PROCESS_SETTLEMENT_TIMEOUT;
        wait_for_root_exit(&mut self.ownership, self.root, deadline);
        settle_descendants(&mut self.ownership, deadline);
        self.ownership.tree_mut().retire_session_for_test();
        let outcome = self.ownership.wait_until_reaped();
        assert!(
            outcome.fault.is_none(),
            "{PROCESS_SETTLEMENT_FAULT_DETAIL}: {:?}",
            outcome.fault
        );
        self.ownership.disarm();
        assert!(!self.root.matches(), "{PROCESS_SETTLEMENT_MISSING_DETAIL}");
    }
}

pub(super) fn wait_for_marker(path: &Path) {
    let deadline = Instant::now() + MARKER_TIMEOUT;
    while !path.exists() {
        assert!(
            Instant::now() < deadline,
            "{MARKER_MISSING_DETAIL}: {}",
            path.display()
        );
        std::thread::sleep(MARKER_POLL_INTERVAL);
    }
}

fn wait_for_tree_members(tree: &mut CapturedProcessTree, minimum: usize) {
    let deadline = Instant::now() + MARKER_TIMEOUT;
    while tree.identities().len() < minimum {
        assert!(Instant::now() < deadline, "{TREE_CAPTURE_MISSING_DETAIL}");
        std::thread::sleep(MARKER_POLL_INTERVAL);
        tree.refresh().unwrap();
    }
}

fn wait_for_root_exit(ownership: &mut PtyOwnership, root: ProcessIdentity, deadline: Instant) {
    let mut observer = WaitidRootExitObserver::new(root);
    loop {
        if let Some(proof) = ownership.poll_root_exit(&mut observer).unwrap() {
            ownership.mark_root_exited_unreaped(proof).unwrap();
            return;
        }
        assert!(
            Instant::now() < deadline,
            "{PROCESS_SETTLEMENT_MISSING_DETAIL}"
        );
        std::thread::sleep(MARKER_POLL_INTERVAL);
    }
}

fn settle_descendants(ownership: &mut PtyOwnership, deadline: Instant) {
    loop {
        ownership
            .tree_mut()
            .terminate_descendants(libc::SIGKILL)
            .unwrap();
        if ownership.tree_mut().live_descendants().unwrap().is_empty() {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "{PROCESS_SETTLEMENT_MISSING_DETAIL}"
        );
        std::thread::sleep(MARKER_POLL_INTERVAL);
    }
}
