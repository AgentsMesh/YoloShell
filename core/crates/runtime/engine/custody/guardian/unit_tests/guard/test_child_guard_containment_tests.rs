use super::pty::RetainedPtyChild;
use super::release_after_cleanup;
use crate::process::{OwnedPtyChild, ProcessIdentity};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

const CLEANUP_PANIC: &str = "injected guarded PTY child cleanup panic";
const HANDOFF_TIMEOUT_DETAIL: &str = "guarded PTY child did not reach its reserved reaper";
const TEST_EXIT_CODE: u32 = 0;
const TEST_PROCESS_ID: u32 = u32::MAX;
const TEST_PROCESS_START: u64 = u64::MAX;
const WAIT_POLL: std::time::Duration = crate::session_timing::GRACEFUL_TERMINATION_POLL_INTERVAL;
const WAIT_TIMEOUT: std::time::Duration = crate::session_timing::FORCE_TERMINATION_CONFIRM_TIMEOUT;

#[derive(Clone, Debug)]
struct ObservedChild {
    terminated: Arc<AtomicBool>,
}

impl portable_pty::ChildKiller for ObservedChild {
    fn kill(&mut self) -> std::io::Result<()> {
        self.terminated.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn clone_killer(&self) -> Box<dyn portable_pty::ChildKiller + Send + Sync> {
        Box::new(self.clone())
    }
}

impl portable_pty::Child for ObservedChild {
    fn try_wait(&mut self) -> std::io::Result<Option<portable_pty::ExitStatus>> {
        Ok(self
            .terminated
            .load(Ordering::SeqCst)
            .then(|| portable_pty::ExitStatus::with_exit_code(TEST_EXIT_CODE)))
    }

    fn wait(&mut self) -> std::io::Result<portable_pty::ExitStatus> {
        Ok(portable_pty::ExitStatus::with_exit_code(TEST_EXIT_CODE))
    }

    fn process_id(&self) -> Option<u32> {
        Some(TEST_PROCESS_ID)
    }
}

#[test]
fn cleanup_panic_still_hands_pty_child_to_the_reserved_reaper() {
    let terminated = Arc::new(AtomicBool::new(false));
    let child: OwnedPtyChild = Box::new(ObservedChild {
        terminated: Arc::clone(&terminated),
    });
    let identity = ProcessIdentity {
        pid: TEST_PROCESS_ID,
        start_time: TEST_PROCESS_START,
    };
    let mut retained = Some(RetainedPtyChild::for_test(child, identity));

    release_after_cleanup(&mut retained, || panic!("{CLEANUP_PANIC}"));

    assert!(retained.is_none());
    let deadline = Instant::now() + WAIT_TIMEOUT;
    while !terminated.load(Ordering::SeqCst) {
        assert!(Instant::now() < deadline, "{HANDOFF_TIMEOUT_DETAIL}");
        std::thread::sleep(WAIT_POLL);
    }
}
