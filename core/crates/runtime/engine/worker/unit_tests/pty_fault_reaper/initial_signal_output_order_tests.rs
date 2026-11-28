use super::*;
use std::cell::{Cell, RefCell};

const OUTPUT_RESUME_REJECTED: &str = "injected output resume failure";

#[derive(Debug, Eq, PartialEq)]
enum SignalStep {
    OutputResumed,
    SignalSent,
}

#[test]
fn graceful_dispatch_resumes_output_before_sending_signal() {
    let steps = RefCell::new(Vec::new());
    assert!(
        initial_signal::dispatch(
            initial_signal::InitialRootSignal::Graceful,
            || {
                steps.borrow_mut().push(SignalStep::OutputResumed);
                Ok(())
            },
            || {
                steps.borrow_mut().push(SignalStep::SignalSent);
                Ok(true)
            }
        )
        .unwrap()
    );
    assert_eq!(
        steps.into_inner(),
        [SignalStep::OutputResumed, SignalStep::SignalSent]
    );
}

#[test]
fn output_resume_failure_does_not_authorize_the_graceful_signal() {
    let sent = Cell::new(false);
    let error = initial_signal::dispatch(
        initial_signal::InitialRootSignal::Graceful,
        || anyhow::bail!(OUTPUT_RESUME_REJECTED),
        || {
            sent.set(true);
            Ok(true)
        },
    )
    .unwrap_err();
    assert_eq!(error.to_string(), OUTPUT_RESUME_REJECTED);
    assert!(!sent.get());
}

#[test]
fn force_dispatch_keeps_its_existing_signal_path() {
    let sent = Cell::new(false);
    assert!(
        initial_signal::dispatch(
            initial_signal::InitialRootSignal::Force,
            || panic!(),
            || {
                sent.set(true);
                Ok(true)
            }
        )
        .unwrap()
    );
    assert!(sent.get());
}

#[test]
fn initial_graceful_signal_advances_output_before_hangup() {
    let (root, mut ownership) = waitable_ownership();
    let _cleanup = LiveAuthorityCleanup { root, armed: true };
    ownership.custody_mut().access = crate::worker::pty_ownership::PtyCustodyAccess::Control;
    let observed = Arc::new(AtomicUsize::new(usize::default()));
    ownership.custody_mut().output_progress =
        FaultOutput::Reader(ReaderBackedPtyOutput::observed(Arc::clone(&observed)));

    initial_signal::request(&mut ownership);

    assert!(observed.load(Ordering::Acquire) > usize::default());
}

#[test]
fn initial_graceful_signal_also_advances_exclusive_output() {
    let (root, mut ownership) = waitable_ownership();
    let _cleanup = LiveAuthorityCleanup { root, armed: true };
    ownership.custody_mut().access = crate::worker::pty_ownership::PtyCustodyAccess::Control;
    let observed = Arc::new(AtomicUsize::new(usize::default()));
    ownership.custody_mut().output_progress = FaultOutput::Exclusive(
        crate::worker::pty_output_progress::ExclusivePtyOutput::observed(Arc::clone(&observed)),
    );
    initial_signal::request(&mut ownership);
    assert!(observed.load(Ordering::Acquire) > usize::default());
}

#[test]
fn missing_root_authority_cannot_advance_output_or_issue_a_signal() {
    let mut ownership = passive_ownership();
    let observed = Arc::new(AtomicUsize::new(usize::default()));
    ownership.custody_mut().output_progress =
        FaultOutput::Reader(ReaderBackedPtyOutput::observed(Arc::clone(&observed)));
    initial_signal::request(&mut ownership);
    assert_eq!(observed.load(Ordering::Acquire), usize::default());
    assert!(ownership.custody().authority.is_none());
}
