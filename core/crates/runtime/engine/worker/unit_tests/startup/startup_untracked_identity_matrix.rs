use super::*;
use crate::process::ProcessIdentity;
use crate::worker::pty_missing_eof::MissingEofDeadline;
use crate::worker::pty_output_progress::ExclusivePtyOutput;
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

const UNTRACKED_IDENTITY_EXECUTABLE: &str = "/bin/sleep";
const UNTRACKED_IDENTITY_DURATION: &str = "30";
const UNTRACKED_IDENTITY_MATCH_FAILURE: &str = "injected untracked identity match failure";

struct UntrackedIdentityCleanup {
    root: ProcessIdentity,
    armed: bool,
}

impl Drop for UntrackedIdentityCleanup {
    fn drop(&mut self) {
        if self.armed {
            let _ = self.root.signal(libc::SIGCONT);
            let _ = self.root.signal(libc::SIGKILL);
            let _ = crate::process::wait_direct_child(self.root.pid);
        }
    }
}

#[test]
fn inline_driver_covers_live_lost_error_and_missing_gate_actions() {
    let child = crate::process::spawn_waitable_test_child(
        Command::new(UNTRACKED_IDENTITY_EXECUTABLE).arg(UNTRACKED_IDENTITY_DURATION),
    );
    let root = ProcessIdentity::capture(child.id()).unwrap();
    let mut child_cleanup = UntrackedIdentityCleanup { root, armed: true };
    let output_calls = Arc::new(AtomicUsize::new(usize::default()));
    let mut guard = StartupChildGuard::with_output(
        Box::new(child),
        ExclusivePtyOutput::observed(Arc::clone(&output_calls)),
    );
    let mut output = guard.output_progress.take().unwrap();
    let mut reader = None;
    let mut missing_eof = MissingEofDeadline::production();
    let mut failure = None;
    let mut authority_lost = false;

    assert!(!advance(
        &mut guard,
        &mut failure,
        &mut authority_lost,
        &mut output,
        &mut reader,
        &mut missing_eof,
    ));
    assert!(guard.live_identity.is_some());
    authority_lost = true;
    let match_failure =
        crate::process::install_test_match_failure(UNTRACKED_IDENTITY_MATCH_FAILURE);
    assert!(!advance(
        &mut guard,
        &mut failure,
        &mut authority_lost,
        &mut output,
        &mut reader,
        &mut missing_eof,
    ));
    drop(match_failure);
    assert!(!advance(
        &mut guard,
        &mut failure,
        &mut authority_lost,
        &mut output,
        &mut reader,
        &mut missing_eof,
    ));

    guard.child.as_mut().unwrap().kill().unwrap();
    guard.child.as_mut().unwrap().wait().unwrap();
    child_cleanup.armed = false;
    guard.unreleased_gate = None;
    assert!(!advance(
        &mut guard,
        &mut failure,
        &mut authority_lost,
        &mut output,
        &mut reader,
        &mut missing_eof,
    ));
    guard.unreleased_gate = Some(crate::worker::UnreleasedBootstrapGate);
    assert!(advance(
        &mut guard,
        &mut failure,
        &mut authority_lost,
        &mut output,
        &mut reader,
        &mut missing_eof,
    ));

    assert!(guard.untracked_gone_committed);
    assert!(output_calls.load(Ordering::Acquire) > usize::default());
    guard.clear();
}

fn advance(
    guard: &mut StartupChildGuard,
    failure: &mut Option<anyhow::Error>,
    authority_lost: &mut bool,
    output: &mut StartupOutputProgress,
    reader: &mut Option<crate::worker::pty_output_progress::PtyOutputRecovery>,
    missing_eof: &mut MissingEofDeadline,
) -> bool {
    guard.advance_untracked_for_test(failure, authority_lost, output, reader, missing_eof)
}
