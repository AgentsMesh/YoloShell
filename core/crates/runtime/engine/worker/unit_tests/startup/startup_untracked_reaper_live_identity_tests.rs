use super::*;
use crate::process::ProcessIdentity;
use std::process::Command;

const LIVE_IDENTITY_EXECUTABLE: &str = "/bin/sleep";
const LIVE_IDENTITY_DURATION: &str = "30";

struct LiveIdentityCleanup {
    root: ProcessIdentity,
    armed: bool,
}

impl Drop for LiveIdentityCleanup {
    fn drop(&mut self) {
        if self.armed {
            let _ = self.root.signal(libc::SIGCONT);
            let _ = self.root.signal(libc::SIGKILL);
            let _ = crate::process::wait_direct_child(self.root.pid);
        }
    }
}

#[test]
fn driver_binds_live_identity_then_proves_loss_without_wait_authority() {
    let child = crate::process::spawn_waitable_test_child(
        Command::new(LIVE_IDENTITY_EXECUTABLE).arg(LIVE_IDENTITY_DURATION),
    );
    let root = ProcessIdentity::capture(child.id()).unwrap();
    let mut child_cleanup = LiveIdentityCleanup { root, armed: true };
    let output_progressed = Arc::new(AtomicUsize::new(usize::default()));
    let mut cleanup = StartupUntrackedCleanup::closed_gate_for_test(
        Box::new(child),
        StartupOutputProgress::Exclusive(ExclusivePtyOutput::observed(Arc::clone(
            &output_progressed,
        ))),
    );

    wait_until(|| {
        assert!(!cleanup.drive_once_for_test());
        cleanup.identity.live().is_some()
    });
    assert!(cleanup.identity.live().is_some());
    cleanup.wait_authority_lost = true;
    assert!(!cleanup.drive_once_for_test());

    cleanup.child.kill().unwrap();
    cleanup.child.wait().unwrap();
    child_cleanup.armed = false;
    assert!(cleanup.drive_once_for_test());
    assert!(cleanup.producers_gone_committed);
}

#[test]
fn durable_identity_proves_loss_when_live_capture_is_unavailable() {
    let child = crate::process::spawn_waitable_test_child(
        Command::new(LIVE_IDENTITY_EXECUTABLE).arg(LIVE_IDENTITY_DURATION),
    );
    let root = ProcessIdentity::capture(child.id()).unwrap();
    let mut child_cleanup = LiveIdentityCleanup { root, armed: true };
    let output_progressed = Arc::new(AtomicUsize::new(usize::default()));
    let mut cleanup = StartupUntrackedCleanup::closed_gate_for_test(
        Box::new(child),
        StartupOutputProgress::Exclusive(ExclusivePtyOutput::observed(Arc::clone(
            &output_progressed,
        ))),
    );
    cleanup.identity.install_durable_for_test(root);
    cleanup.wait_authority_lost = true;

    assert!(cleanup.identity.live().is_none());
    assert!(!cleanup.drive_once_for_test());
    cleanup.child.kill().unwrap();
    cleanup.child.wait().unwrap();
    child_cleanup.armed = false;
    assert!(cleanup.drive_once_for_test());
    assert!(cleanup.producers_gone_committed);
}
