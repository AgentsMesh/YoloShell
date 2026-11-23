use super::*;

const TEST_FAULT_DETAIL: &str = "injected fault-reap observer failure";
const STATUS_MISSING_DETAIL: &str = "fault-reap observer status is missing";
const PROGRESS_MISSING_DETAIL: &str = "fault-reap observer progress is missing";

#[test]
fn default_observer_has_no_active_fault_reap() {
    let observer = FaultReapStateObserver::default();

    assert!(observer.status().is_none());
    assert!(observer.shares_state_with(&observer.clone()));
    assert!(!observer.shares_state_with(&FaultReapStateObserver::default()));
}

#[test]
fn observer_publishes_monotonic_fault_reap_lifecycle() {
    let observer = FaultReapStateObserver::default();
    observer.publish_handoff(None, PtyCustodyAccess::Control);
    let handoff = observer.status().expect(STATUS_MISSING_DETAIL);
    let handoff_progress = handoff
        .last_semantic_progress
        .expect(PROGRESS_MISSING_DETAIL);
    assert!(handoff.custody_retained);
    assert!(!handoff.observer_only);
    assert_eq!(handoff_progress.sequence, PROGRESS_SEQUENCE_INCREMENT);
    assert!(handoff_progress.observed_at_ms >= 0);
    assert_eq!(handoff_progress.kind, FaultReapProgressKind::Handoff);

    observer.publish_progress(FaultReapProgressKind::CleanupAdvanced);
    let cleanup = observer.status().expect(STATUS_MISSING_DETAIL);
    let cleanup_progress = cleanup
        .last_semantic_progress
        .expect(PROGRESS_MISSING_DETAIL);
    assert_eq!(
        cleanup_progress.sequence,
        handoff_progress.sequence + PROGRESS_SEQUENCE_INCREMENT
    );
    assert_eq!(
        cleanup_progress.kind,
        FaultReapProgressKind::CleanupAdvanced
    );

    observer.publish_observer_only();
    let observer_only = observer.status().expect(STATUS_MISSING_DETAIL);
    let observer_progress = observer_only
        .last_semantic_progress
        .expect(PROGRESS_MISSING_DETAIL);
    assert!(observer_only.custody_retained);
    assert!(observer_only.observer_only);
    assert_eq!(
        observer_progress.sequence,
        cleanup_progress.sequence + PROGRESS_SEQUENCE_INCREMENT
    );
    assert_eq!(observer_progress.kind, FaultReapProgressKind::ObserverOnly);

    observer.publish_fault(&anyhow::anyhow!(TEST_FAULT_DETAIL));
    assert_eq!(
        observer
            .status()
            .expect(STATUS_MISSING_DETAIL)
            .last_fault
            .as_deref(),
        Some(TEST_FAULT_DETAIL)
    );

    observer.publish_terminal();
    let completed = observer.status().expect(STATUS_MISSING_DETAIL);
    let completed_progress = completed
        .last_semantic_progress
        .expect(PROGRESS_MISSING_DETAIL);
    assert!(!completed.custody_retained);
    assert!(completed.observer_only);
    assert_eq!(
        completed_progress.sequence,
        observer_progress.sequence + PROGRESS_SEQUENCE_INCREMENT
    );
    assert_eq!(completed_progress.kind, FaultReapProgressKind::Completed);
    assert_eq!(completed.last_fault.as_deref(), Some(TEST_FAULT_DETAIL));
}

#[test]
fn observer_retains_a_fault_before_handoff_and_ignores_other_early_publications() {
    let observer = FaultReapStateObserver::default();
    observer.publish_progress(FaultReapProgressKind::CleanupAdvanced);
    observer.publish_observer_only();
    observer.publish_fault(&anyhow::anyhow!(TEST_FAULT_DETAIL));
    observer.publish_terminal();
    assert!(observer.status().is_none());

    observer.publish_handoff(None, PtyCustodyAccess::Control);
    observer.publish_handoff(
        Some(&anyhow::anyhow!(TEST_FAULT_DETAIL)),
        PtyCustodyAccess::ObserverOnly,
    );
    let status = observer.status().expect(STATUS_MISSING_DETAIL);
    assert_eq!(
        status
            .last_semantic_progress
            .expect(PROGRESS_MISSING_DETAIL)
            .sequence,
        PROGRESS_SEQUENCE_INCREMENT
    );
    assert_eq!(status.last_fault.as_deref(), Some(TEST_FAULT_DETAIL));
}

#[test]
fn handoff_failure_takes_precedence_over_a_buffered_fault() {
    const HANDOFF_FAULT_DETAIL: &str = "injected handoff failure";
    let observer = FaultReapStateObserver::default();
    observer.publish_fault(&anyhow::anyhow!(TEST_FAULT_DETAIL));

    observer.publish_handoff(
        Some(&anyhow::anyhow!(HANDOFF_FAULT_DETAIL)),
        PtyCustodyAccess::Control,
    );

    assert_eq!(
        observer
            .status()
            .expect(STATUS_MISSING_DETAIL)
            .last_fault
            .as_deref(),
        Some(HANDOFF_FAULT_DETAIL)
    );
}
