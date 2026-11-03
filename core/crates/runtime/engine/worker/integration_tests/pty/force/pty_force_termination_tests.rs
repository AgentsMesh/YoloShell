use super::OPERATION_CALL_INCREMENT;
use super::{
    EmptyClosureOutputRequirement, FenceBehavior, ForceOutput, ForceReady, ForceTerminationPoll,
    OutputFence, ProgressBehavior, PtyForceTermination, ScriptedForceOutput,
};
use crate::process::{
    CapturedProcessTree, DetachedStandardError, FrozenProcessTree, ProcessClosureVersion,
    ProcessIdentity, UnreapedRootExit, spawn_detached,
};
use crate::worker::pty_output_progress::OutputProgressState;
use crate::worker::pty_process_custody::PtyProcessCustody;
use anyhow::Result;
use std::path::Path;
use std::process::Child;
use std::time::{Duration, Instant};

const SLEEP_EXECUTABLE: &str = "/bin/sleep";
const SLEEP_DURATION: &str = "30";
const TRANSITION_TIMEOUT: Duration = Duration::from_secs(5);
const TRANSITION_POLL_INTERVAL: Duration = Duration::from_millis(10);
const TRANSITION_TIMEOUT_DETAIL: &str = "force transaction transition timed out";
const UNEXPECTED_FORCE_ERROR_DETAIL: &str = "force transaction returned an unexpected error";
const UNEXPECTED_FORCE_COMPLETION_DETAIL: &str = "force transaction completed unexpectedly";
struct ChildGuard {
    child: Option<Child>,
    root: ProcessIdentity,
}

impl ChildGuard {
    fn spawn() -> (Self, CapturedProcessTree) {
        let child = spawn_detached(
            Path::new(SLEEP_EXECUTABLE),
            &[SLEEP_DURATION.into()],
            DetachedStandardError::Null,
        )
        .unwrap();
        let root = ProcessIdentity::capture(child.id()).unwrap();
        let mut tree = CapturedProcessTree::capture(root).unwrap();
        tree.authorize_destructive_for_test();
        (
            Self {
                child: Some(child),
                root,
            },
            tree,
        )
    }

    fn reap(&mut self) {
        self.child.take().unwrap().wait().unwrap();
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.root.signal(libc::SIGCONT);
        let _ = self.root.signal(libc::SIGKILL);
        if let Some(mut child) = self.child.take() {
            let _ = child.wait();
        }
    }
}

fn poll_force(
    force: &mut PtyForceTermination,
    custody: &mut Option<PtyProcessCustody>,
    output: &mut ScriptedForceOutput,
) -> Result<ForceTerminationPoll> {
    force.poll(custody, &mut ForceOutput::Scripted(output))
}

include!("../../../test_support/integration/pty/pty_force_termination_test_support.rs");
include!("pty_force_termination_state_tests.rs");
include!("pty_force_termination_gone_tests.rs");
include!("pty_force_termination_empty_closure_tests.rs");
include!("../../../test_support/integration/pty/pty_force_termination_eof_test_support.rs");
include!("pty_force_termination_eof_tests.rs");
include!("pty_force_termination_eof_commit_tests.rs");
#[cfg(target_os = "macos")]
include!("pty_force_termination_process_commit_tests.rs");
include!("pty_force_termination_unwind_tests.rs");
#[cfg(target_os = "macos")]
include!("pty_force_termination_handoff_tests.rs");
#[cfg(target_os = "macos")]
include!("pty_force_termination_race_macos_tests.rs");
