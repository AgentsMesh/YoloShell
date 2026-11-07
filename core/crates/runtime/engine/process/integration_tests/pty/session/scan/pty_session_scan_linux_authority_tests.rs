use super::*;
use crate::process::inventory_test_support::{NO_PARENT_PID, TestProcess, entry};
use crate::process::{LiveProcessIdentity, install_test_match_failure};
use std::cell::Cell;
use std::sync::Arc;

const MATCH_POLL_FAILURE_DETAIL: &str = "injected retained pidfd poll failure";
const CACHED_IDENTITY_RECAPTURED_DETAIL: &str = "cached identity was numerically recaptured";
const RETAINED_IDENTITY_RECAPTURED_DETAIL: &str = "retained identity was numerically recaptured";

#[test]
fn exited_cached_match_is_suppressed_then_reevaluated_without_numeric_reuse() {
    let mut process = TestProcess::spawn();
    let (current, state, _) = retained_fixture(&process);
    process.terminate();
    let evaluations = Cell::new(usize::default());

    let (pending, next, batch_consumed) =
        advance_with(&current, state, false, usize::MAX, |_, _| {
            evaluations.set(evaluations.get() + 1);
            EvaluatedCandidate::Excluded
        });

    assert_eq!(pending.status, CapabilityScanStatus::Pending);
    assert!(pending.identities.is_empty());
    assert_eq!(evaluations.get(), usize::default());
    assert_eq!(next.pending, Some(CandidatePending::Exited));
    assert!(next.matched.is_empty());
    assert!(batch_consumed);

    let (complete, _, batch_consumed) = advance_with(&current, next, false, usize::MAX, |_, _| {
        evaluations.set(evaluations.get() + 1);
        EvaluatedCandidate::Excluded
    });
    assert_eq!(complete.status, CapabilityScanStatus::Complete);
    assert_eq!(evaluations.get(), 1);
    assert!(batch_consumed);
}

#[test]
fn cached_poll_fault_retains_authority_but_suppresses_round_evidence() {
    let process = TestProcess::spawn();
    let (current, state, _) = retained_fixture(&process);
    let failure = install_test_match_failure(MATCH_POLL_FAILURE_DETAIL);

    let (pending, next, batch_consumed) =
        advance_with(&current, state, false, usize::MAX, |_, _| {
            panic!("{CACHED_IDENTITY_RECAPTURED_DETAIL}")
        });

    assert_eq!(pending.status, CapabilityScanStatus::Pending);
    assert!(pending.identities.is_empty());
    assert_eq!(
        next.pending,
        Some(CandidatePending::Query(QueryKind::Identity))
    );
    assert_eq!(next.matched.len(), 1);
    assert!(batch_consumed);
    drop(failure);

    let (complete, _, batch_consumed) = advance_with(&current, next, false, usize::MAX, |_, _| {
        panic!("{RETAINED_IDENTITY_RECAPTURED_DETAIL}")
    });
    assert_eq!(complete.status, CapabilityScanStatus::Complete);
    assert_eq!(complete.identities.len(), 1);
    assert!(batch_consumed);
}

#[test]
fn over_capacity_match_is_never_published_as_ephemeral_authority() {
    let process = TestProcess::spawn();
    let (current, mut state, matched) = retained_fixture(&process);
    state.matched.clear();

    let (pending, next, batch_consumed) =
        advance_with(&current, state, false, usize::default(), |_, _| {
            EvaluatedCandidate::Matched(matched.clone())
        });

    assert_eq!(pending.status, CapabilityScanStatus::Pending);
    assert!(pending.identities.is_empty());
    assert!(next.matched.is_empty());
    assert_eq!(
        next.pending,
        Some(CandidatePending::Budget(BudgetKind::RetainedAuthorities))
    );
    assert!(batch_consumed);
}

fn retained_fixture(
    process: &TestProcess,
) -> (Vec<InventoryEntry>, LinuxScanState, MatchedCandidate) {
    let durable = process.identity;
    let live = LiveProcessIdentity::capture(durable).unwrap().unwrap();
    let matched = MatchedCandidate { live, durable };
    let current = vec![entry(durable, NO_PARENT_PID, durable.pid)];
    let state = LinuxScanState {
        universe: Arc::from([durable]),
        matched: [(durable, matched.clone())].into_iter().collect(),
        ..LinuxScanState::default()
    };
    (current, state, matched)
}
