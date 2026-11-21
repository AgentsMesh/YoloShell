use super::*;
use crate::process::{CapturedProcessTree, OwnedPtyChild, ProcessIdentity, RootWaitAuthority};
use crate::worker::pty_output_progress::ExclusivePtyOutput;
use crate::worker::pty_ownership::PtyCustodyAccess;
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

const EXCLUSIVE_SIGNAL_TEST_COMMAND: &str = "/bin/sleep";
const EXCLUSIVE_SIGNAL_TEST_DURATION: &str = "30";

#[path = "explicit_bootstrap_initial_signal_tests.rs"]
mod explicit_bootstrap_failure;

#[test]
fn exclusive_output_cannot_hang_up_a_live_root_before_the_force_output_fence() {
    let (root, mut ownership) = ownership();
    let _cleanup = ChildCleanup { root };
    let observed = Arc::new(AtomicUsize::default());
    ownership.custody_mut().output_progress =
        FaultOutput::Exclusive(ExclusivePtyOutput::observed(Arc::clone(&observed)));
    assert!(
        !ownership
            .custody()
            .processes
            .as_ref()
            .unwrap()
            .requires_exact_root_force_fallback()
    );

    assert!(!request(&mut ownership));

    assert!(observed.load(Ordering::Acquire) > usize::default());
    assert!(ownership.failure.is_none());
    assert!(root.matches());
    assert!(ownership.custody().force.allows_graceful_fault_handoff());
}

#[test]
fn unavailable_force_authority_keeps_the_exact_root_graceful_fallback() {
    let (root, mut ownership) = ownership();
    let _cleanup = ChildCleanup { root };
    let observed = Arc::new(AtomicUsize::default());
    let custody = ownership.custody_mut();
    custody.output_progress =
        FaultOutput::Exclusive(ExclusivePtyOutput::observed(Arc::clone(&observed)));
    custody
        .processes
        .as_mut()
        .unwrap()
        .tracking_mut()
        .deny_destructive_for_test();

    assert!(request(&mut ownership));

    assert!(observed.load(Ordering::Acquire) > usize::default());
    assert!(ownership.failure.is_none());
}

#[test]
fn explicit_exact_root_force_fallback_does_not_resume_exclusive_output() {
    let (root, mut ownership) = ownership();
    let _cleanup = ChildCleanup { root };
    let observed = Arc::new(AtomicUsize::default());
    let custody = ownership.custody_mut();
    custody.output_progress =
        FaultOutput::Exclusive(ExclusivePtyOutput::observed(Arc::clone(&observed)));
    custody
        .processes
        .as_mut()
        .unwrap()
        .tracking_mut()
        .deny_destructive_for_test();
    custody.force.record_force_request(true);

    assert!(request(&mut ownership));

    assert_eq!(observed.load(Ordering::Acquire), usize::default());
    assert!(ownership.failure.is_none());
}

fn ownership() -> (ProcessIdentity, FaultReap) {
    let child = crate::process::spawn_waitable_test_child(
        Command::new(EXCLUSIVE_SIGNAL_TEST_COMMAND).arg(EXCLUSIVE_SIGNAL_TEST_DURATION),
    );
    let root = ProcessIdentity::capture(child.id()).unwrap();
    let tree = CapturedProcessTree::capture(root).unwrap();
    let child = Box::new(child) as OwnedPtyChild;
    let authority = RootWaitAuthority::prove_owned_child(&child, &tree)
        .unwrap()
        .bind(child);
    let mut ownership = super::super::super::fault_reap_construction::passive_fault_reap_for_test();
    let custody = ownership.custody_mut();
    custody.authority = Some(authority);
    custody.processes = Some(PtyProcessCustody::tracking(tree));
    custody.access = PtyCustodyAccess::Control;
    (root, ownership)
}

struct ChildCleanup {
    root: ProcessIdentity,
}

impl Drop for ChildCleanup {
    fn drop(&mut self) {
        let _ = self.root.signal(libc::SIGKILL);
        let _ = crate::process::wait_direct_child(self.root.pid);
    }
}
