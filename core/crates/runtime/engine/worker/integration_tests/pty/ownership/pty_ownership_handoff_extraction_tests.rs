use super::*;
use crate::process::{CapturedProcessTree, ProcessIdentity};
use std::path::Path;
use std::time::{Duration, Instant};

const CHILD_EXECUTABLE: &str = "/bin/sleep";
const CHILD_DURATION: &str = "30";
const EXPECTED_EXTRACTION_PANIC: &str = "PTY extraction panic was not injected";
const INJECTED_EXTRACTION_PANIC: &str = "injected PTY ownership handoff extraction panic";
const HANDOFF_FAILURE_DETAIL: &str = "injected PTY ownership handoff failure";
const WAIT_INTERVAL: Duration = Duration::from_millis(5);
const WAIT_TIMEOUT: Duration = Duration::from_secs(10);

#[test]
fn every_extraction_boundary_rolls_back_complete_live_custody() {
    for &phase in super::handoff_extraction::HandoffExtractionPhase::ALL {
        assert_boundary_rolls_back(phase);
    }
}

#[test]
fn ownership_drop_preserves_completion_after_every_extraction_boundary() {
    for &phase in super::handoff_extraction::HandoffExtractionPhase::ALL {
        assert_drop_boundary_recovers(phase);
    }
}

fn assert_drop_boundary_recovers(phase: super::handoff_extraction::HandoffExtractionPhase) {
    let child = crate::process::spawn_detached(
        Path::new(CHILD_EXECUTABLE),
        &[CHILD_DURATION.into()],
        crate::process::DetachedStandardError::Null,
    )
    .unwrap();
    let identity = ProcessIdentity::capture(child.id()).unwrap();
    let tree = CapturedProcessTree::capture(identity).unwrap();
    let mut ownership =
        PtyOwnership::new_for_test(Box::new(child), tree, reserve_fault_reaper().unwrap()).unwrap();
    ownership.panic_after_handoff_extraction_for_test(phase);
    let (completion, receiver) = FaultReapCompletion::owner_channel();

    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        ownership
            .settle_after_fault_with_failure(completion, anyhow::anyhow!(HANDOFF_FAILURE_DETAIL))
    }));
    let payload = panic.err().expect(EXPECTED_EXTRACTION_PANIC);
    assert_eq!(
        panic_message(payload.as_ref()),
        Some(INJECTED_EXTRACTION_PANIC)
    );
    drop(ownership);

    let outcome = receiver.receive_timeout(WAIT_TIMEOUT).unwrap();
    assert_eq!(outcome.evidence, ReapEvidence::Waited);
    assert!(outcome.status.is_some());
    assert_handoff_failure(&outcome);
    wait_until_gone(identity);
}

fn assert_boundary_rolls_back(phase: super::handoff_extraction::HandoffExtractionPhase) {
    let child = crate::process::spawn_detached(
        Path::new(CHILD_EXECUTABLE),
        &[CHILD_DURATION.into()],
        crate::process::DetachedStandardError::Null,
    )
    .unwrap();
    let identity = ProcessIdentity::capture(child.id()).unwrap();
    let tree = CapturedProcessTree::capture(identity).unwrap();
    let mut ownership =
        PtyOwnership::new_for_test(Box::new(child), tree, reserve_fault_reaper().unwrap()).unwrap();
    ownership.panic_after_handoff_extraction_for_test(phase);
    let (failed_completion, failed_receiver) = FaultReapCompletion::owner_channel();

    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        ownership.settle_after_fault_with_failure(
            failed_completion,
            anyhow::anyhow!(HANDOFF_FAILURE_DETAIL),
        )
    }));

    let payload = panic.err().expect(EXPECTED_EXTRACTION_PANIC);
    assert_eq!(
        panic_message(payload.as_ref()),
        Some(INJECTED_EXTRACTION_PANIC)
    );
    let (completion, replacement_receiver) = FaultReapCompletion::owner_channel();
    let settlement = ownership.settle_after_fault(completion);
    assert!(matches!(settlement, FaultSettlement::Deferred(_)));
    let outcome = failed_receiver.receive_timeout(WAIT_TIMEOUT).unwrap();
    assert_eq!(outcome.evidence, ReapEvidence::Waited);
    assert!(outcome.status.is_some());
    assert_handoff_failure(&outcome);
    assert!(matches!(
        replacement_receiver.receive_timeout(WAIT_TIMEOUT),
        Err(std::sync::mpsc::RecvTimeoutError::Disconnected)
    ));
    wait_until_gone(identity);
}

fn assert_handoff_failure(outcome: &ReapOutcome) {
    assert_eq!(
        outcome.fault.as_ref().map(ToString::to_string).as_deref(),
        Some(HANDOFF_FAILURE_DETAIL)
    );
}

fn wait_until_gone(identity: ProcessIdentity) {
    let deadline = Instant::now() + WAIT_TIMEOUT;
    while identity.matches() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(WAIT_INTERVAL);
    }
}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> Option<&str> {
    payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
}
