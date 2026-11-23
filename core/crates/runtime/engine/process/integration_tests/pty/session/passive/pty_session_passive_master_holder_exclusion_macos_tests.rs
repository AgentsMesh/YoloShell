use super::*;
use crate::process::inventory_snapshot::InventoryEntry;

const GUARDIAN_LINEAGE: u64 = 82_001;
const HOLDER_LINEAGE: u64 = 82_002;
const OTHER_LINEAGE: u64 = 82_003;
const HOLDER_PID: u32 = 83_001;
const OTHER_PID: u32 = 83_002;
const HOLDER_PROCESS_GROUP: u32 = 83_003;
const HOLDER_SESSION: u32 = 83_004;
const UNRELATED_PID: u32 = 83_005;
const NO_PARENT_PID: u32 = 0;
const NO_PARENT_LINEAGE: u64 = 0;
const OLDER_START_TIME_GAP: u64 = 2;
const REUSED_START_TIME_STEP: u64 = 1;

#[test]
fn excluded_exact_worker_and_child_are_still_accounted() {
    let (ancestry, guardian) = incomplete_ancestry();
    let worker = older_identity(HOLDER_PID, guardian);
    let child = older_identity(OTHER_PID, guardian);
    let exclusions = MacosPtyHolderExclusions { worker, child };
    let entries = [entry(worker, HOLDER_LINEAGE), entry(child, OTHER_LINEAGE)];
    let before = HashSet::from([worker.pid, child.pid]);

    let candidates = classify_holder_candidates(&before, &entries, &ancestry, &exclusions).unwrap();

    assert!(candidates.is_empty());
}

#[test]
fn reused_excluded_pid_is_classified_and_fails_closed() {
    let (ancestry, guardian) = incomplete_ancestry();
    let worker = older_identity(HOLDER_PID, guardian);
    let child = older_identity(OTHER_PID, guardian);
    let exclusions = MacosPtyHolderExclusions { worker, child };
    let reused = ProcessIdentity {
        pid: worker.pid,
        start_time: worker.start_time.saturating_add(REUSED_START_TIME_STEP),
    };

    assert!(
        classify_holder_candidates(
            &HashSet::from([reused.pid]),
            &[entry(reused, HOLDER_LINEAGE)],
            &ancestry,
            &exclusions,
        )
        .is_err()
    );
}

#[test]
fn unrelated_older_ambiguous_holder_fails_closed() {
    let (ancestry, guardian) = incomplete_ancestry();
    let worker = older_identity(HOLDER_PID, guardian);
    let child = older_identity(OTHER_PID, guardian);
    let exclusions = MacosPtyHolderExclusions { worker, child };
    let unrelated = older_identity(UNRELATED_PID, guardian);

    assert!(
        classify_holder_candidates(
            &HashSet::from([unrelated.pid]),
            &[entry(unrelated, OTHER_LINEAGE)],
            &ancestry,
            &exclusions,
        )
        .is_err()
    );
}

#[test]
fn holder_universe_change_fails_closed() {
    let before = HashSet::from([HOLDER_PID]);
    let after = HashSet::from([OTHER_PID]);

    let error = verify_holder_universe(&before, &after).unwrap_err();

    assert_eq!(error.to_string(), HOLDER_UNIVERSE_CHANGED_DETAIL);
}

fn incomplete_ancestry() -> (
    super::super::holder_lineage::GuardianAncestry,
    ProcessIdentity,
) {
    let guardian = ProcessIdentity::capture(std::process::id()).unwrap();
    let ancestry = super::super::holder_lineage::GuardianAncestry::incomplete_for_test(
        guardian,
        GUARDIAN_LINEAGE,
        HashSet::from([GUARDIAN_LINEAGE]),
    );
    (ancestry, guardian)
}

fn older_identity(pid: u32, guardian: ProcessIdentity) -> ProcessIdentity {
    ProcessIdentity {
        pid,
        start_time: guardian.start_time.saturating_sub(OLDER_START_TIME_GAP),
    }
}

fn entry(identity: ProcessIdentity, lineage: u64) -> InventoryEntry {
    InventoryEntry {
        pid: identity.pid,
        parent_pid: NO_PARENT_PID,
        process_group: HOLDER_PROCESS_GROUP,
        session_id: HOLDER_SESSION,
        start_time: identity.start_time,
        unique_id: lineage,
        parent_unique_id: NO_PARENT_LINEAGE,
    }
}
