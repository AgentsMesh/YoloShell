use super::*;
use std::cell::Cell;
use std::collections::HashSet;

const FIRST_SYNTHETIC_PID: u32 = 101;
const FIRST_SYNTHETIC_START_TIME: u64 = 211;
const LARGE_UNIVERSE_SIZE: usize = 300;
const MAX_TEST_ADVANCES: usize = 16;
const TEST_SESSION_ID: u32 = 7;

#[test]
fn large_excluded_universe_completes_with_bounded_per_advance_work() {
    let current = universe(LARGE_UNIVERSE_SIZE);
    let identities = current
        .iter()
        .map(|entry| entry.identity())
        .collect::<Vec<_>>();
    let mut state = LinuxScanState {
        universe: Arc::from(identities),
        ..LinuxScanState::default()
    };
    let mut completed = false;
    for _ in 0..MAX_TEST_ADVANCES {
        let evaluated = Cell::new(usize::default());
        let (candidates, next, batch_consumed) =
            advance_with(&current, state, false, usize::MAX, |_entry, _budget| {
                evaluated.set(evaluated.get() + 1);
                EvaluatedCandidate::Excluded
            });
        assert!(evaluated.get() <= LINUX_PTY_PROOF_CANDIDATES_PER_BATCH);
        assert!(batch_consumed);
        state = next;
        if candidates.status == CapabilityScanStatus::Complete {
            completed = true;
            break;
        }
    }
    assert!(completed);
}

#[test]
fn uncommitted_batch_repeats_the_same_candidate_window() {
    let current = universe(LARGE_UNIVERSE_SIZE);
    let state = LinuxScanState::default();
    let first = evaluated_pids(&current, state.clone());
    let repeated = evaluated_pids(&current, state);

    assert_eq!(first, repeated);
    assert_eq!(first.len(), LINUX_PTY_PROOF_CANDIDATES_PER_BATCH);
}

#[test]
fn changed_universe_discards_exclusions_and_restarts_at_its_first_batch() {
    let prior = universe(1);
    let current = universe(LINUX_PTY_PROOF_CANDIDATES_PER_BATCH + 1);
    let prior_identity = prior[0].identity();
    let state = LinuxScanState {
        universe: Arc::from([prior_identity]),
        excluded: HashSet::from([prior_identity]),
        cursor: prior.len(),
        ..LinuxScanState::default()
    };
    let identities = current
        .iter()
        .map(|entry| entry.identity())
        .collect::<Vec<_>>();

    let evaluated = evaluated_pids(&current, state.reconcile(Arc::from(identities)));

    assert_eq!(evaluated.len(), LINUX_PTY_PROOF_CANDIDATES_PER_BATCH);
    assert_eq!(evaluated.first(), Some(&prior_identity.pid));
}

fn evaluated_pids(current: &[InventoryEntry], state: LinuxScanState) -> Vec<u32> {
    let mut evaluated = Vec::new();
    let (_, _, batch_consumed) =
        advance_with(current, state, false, usize::MAX, |entry, _budget| {
            evaluated.push(entry.pid);
            EvaluatedCandidate::Excluded
        });
    assert_eq!(batch_consumed, !current.is_empty());
    evaluated
}

#[test]
fn empty_universe_does_not_claim_finite_batch_progress() {
    let (candidates, _next, batch_consumed) = advance_with(
        &[],
        LinuxScanState::default(),
        false,
        usize::MAX,
        |_entry, _budget| unreachable!(),
    );

    assert_eq!(candidates.status, CapabilityScanStatus::Complete);
    assert!(!batch_consumed);
}

fn universe(count: usize) -> Vec<InventoryEntry> {
    (usize::default()..count)
        .map(|offset| {
            let offset = u32::try_from(offset).unwrap();
            InventoryEntry {
                pid: FIRST_SYNTHETIC_PID + offset,
                parent_pid: u32::default(),
                process_group: FIRST_SYNTHETIC_PID + offset,
                session_id: TEST_SESSION_ID,
                start_time: FIRST_SYNTHETIC_START_TIME + u64::from(offset),
                effective_user_id: unsafe { libc::geteuid() },
            }
        })
        .collect()
}
