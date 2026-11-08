use super::*;
use crate::process::pty_session_proof::macos_released::holder_observation::ReleasedHolderQueryObservation;

const EMPTY_QUERY_COUNT: usize = 0;
const UNBOUND_HOLDER_COUNT: usize = 1;
const HOLDER_PID: u32 = 310_001;
const HOLDER_GENERATION: u64 = 320_001;
const HOLDER_START_TIME: u64 = 330_001;

#[test]
fn custody_query_failure_cannot_complete_the_snapshot() {
    let priority = selection(ReleasedHolderQueryObservation::failed());
    let snapshot = exact(MacosGlobalScanMode::CustodyPtyObject, &priority);

    assert!(snapshot.is_none());
    assert!(!validates(
        MacosGlobalScanMode::CustodyPtyObject,
        snapshot.as_ref(),
        &ReleasedScanState::default(),
    ));
}

#[test]
fn custody_unbound_holder_generation_cannot_complete_the_snapshot() {
    let priority = selection(ReleasedHolderQueryObservation::succeeded(
        UNBOUND_HOLDER_COUNT,
        EMPTY_QUERY_COUNT,
    ));
    let snapshot = exact(MacosGlobalScanMode::CustodyPtyObject, &priority);

    assert!(snapshot.is_none());
    assert!(!validates(
        MacosGlobalScanMode::CustodyPtyObject,
        snapshot.as_ref(),
        &ReleasedScanState::default(),
    ));
}

#[test]
fn custody_successful_empty_query_cannot_grant_completion() {
    let priority = selection(ReleasedHolderQueryObservation::succeeded(
        EMPTY_QUERY_COUNT,
        EMPTY_QUERY_COUNT,
    ));
    let snapshot = exact(MacosGlobalScanMode::CustodyPtyObject, &priority);

    assert!(snapshot.is_none());
    assert!(!validates(
        MacosGlobalScanMode::CustodyPtyObject,
        snapshot.as_ref(),
        &ReleasedScanState::default(),
    ));
}

#[test]
fn custody_missing_path_grants_an_exact_empty_snapshot() {
    let priority = selection(ReleasedHolderQueryObservation::missing_path());
    let snapshot = exact(MacosGlobalScanMode::CustodyPtyObject, &priority);

    assert_eq!(snapshot, Some(HashSet::new()));
    assert!(validates(
        MacosGlobalScanMode::CustodyPtyObject,
        snapshot.as_ref(),
        &ReleasedScanState::default(),
    ));
}

#[test]
fn recovery_without_an_exact_snapshot_keeps_the_full_candidate_universe() {
    let holder = candidate();
    for observation in [
        ReleasedHolderQueryObservation::failed(),
        ReleasedHolderQueryObservation::succeeded(EMPTY_QUERY_COUNT, EMPTY_QUERY_COUNT),
        ReleasedHolderQueryObservation::succeeded(UNBOUND_HOLDER_COUNT, EMPTY_QUERY_COUNT),
    ] {
        let priority = selection(observation);
        let snapshot = exact(MacosGlobalScanMode::RecoveryCustodyPtyObject, &priority);
        let candidates = constrain_candidates(Arc::from([holder]), snapshot.as_ref());

        assert!(snapshot.is_none());
        assert_eq!(candidates.as_ref(), [holder]);
        assert!(validates(
            MacosGlobalScanMode::RecoveryCustodyPtyObject,
            snapshot.as_ref(),
            &ReleasedScanState::default(),
        ));
    }
}

#[test]
fn recovery_exact_snapshot_requires_and_retains_its_matching_generation() {
    let holder = candidate();
    let mut priority = selection(ReleasedHolderQueryObservation::succeeded(
        UNBOUND_HOLDER_COUNT,
        UNBOUND_HOLDER_COUNT,
    ));
    priority.observed_holder_identities.insert(holder.live);
    let snapshot = exact(MacosGlobalScanMode::RecoveryCustodyPtyObject, &priority);
    let candidates = constrain_candidates(Arc::from([holder]), snapshot.as_ref());
    let mut matched = ReleasedScanState::default();
    matched.matched.insert(holder.live, holder);

    assert_eq!(candidates.as_ref(), [holder]);
    assert!(!validates(
        MacosGlobalScanMode::RecoveryCustodyPtyObject,
        snapshot.as_ref(),
        &ReleasedScanState::default(),
    ));
    assert!(validates(
        MacosGlobalScanMode::RecoveryCustodyPtyObject,
        snapshot.as_ref(),
        &matched,
    ));
}

fn selection(observation: ReleasedHolderQueryObservation) -> ReleasedPrioritySelection {
    ReleasedPrioritySelection {
        identities: HashSet::new(),
        holder_identities: HashSet::new(),
        observed_holder_identities: HashSet::new(),
        holder_hints: Default::default(),
        observation,
    }
}

fn candidate() -> ReleasedCandidate {
    ReleasedCandidate {
        live: LiveProcessIdentity::from_kernel(HOLDER_PID, HOLDER_GENERATION).unwrap(),
        durable: crate::process::ProcessIdentity {
            pid: HOLDER_PID,
            start_time: HOLDER_START_TIME,
        },
        parent_lineage: u64::default(),
        stable: true,
        capability_root: false,
        environment_eligible: false,
    }
}
