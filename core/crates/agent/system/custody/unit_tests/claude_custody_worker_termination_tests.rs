use super::*;
use crate::claude_custody::types::{CustodyPublication, CustodyState};

const SECOND_TARGET_PID: u32 = 920_001;
const SECOND_TARGET_START: u64 = 920_002;
const OTHER_WORKER_PID: u32 = 920_003;
const FIRST_RECORD: usize = 0;
const SECOND_RECORD: usize = 1;

#[test]
fn worker_claim_transitions_only_active_custody() {
    let fixture = Fixture::new();
    arm(&fixture.ledger, &fixture.spec).unwrap();
    let claim = begin_worker_termination(&fixture.settings, &fixture.spec.worker).unwrap();
    assert_record(
        &fixture,
        FIRST_RECORD,
        CustodyPublication::Pending,
        CustodyState::Armed,
    );
    settle_worker_termination(&fixture.settings, &claim).unwrap();
    assert_record(
        &fixture,
        FIRST_RECORD,
        CustodyPublication::Pending,
        CustodyState::Armed,
    );
}

#[test]
fn exact_worker_claim_contains_and_settles_active_custody() {
    let fixture = Fixture::new();
    arm(&fixture.ledger, &fixture.spec).unwrap();
    set_active(&fixture);
    let claim = begin_worker_termination(&fixture.settings, &fixture.spec.worker).unwrap();
    assert_record(
        &fixture,
        FIRST_RECORD,
        CustodyPublication::Active,
        CustodyState::Containing,
    );
    settle_worker_termination(&fixture.settings, &claim).unwrap();
    assert_record(
        &fixture,
        FIRST_RECORD,
        CustodyPublication::Active,
        CustodyState::Settled,
    );
}

#[test]
fn final_settlement_captures_late_active_record_for_claimed_worker() {
    let fixture = Fixture::new();
    arm(&fixture.ledger, &fixture.spec).unwrap();
    set_active(&fixture);
    let claim = begin_worker_termination(&fixture.settings, &fixture.spec.worker).unwrap();
    let mut late = fixture.spec.clone();
    late.invocation_id = uuid::Uuid::new_v4();
    late.target.pid = SECOND_TARGET_PID;
    late.target.start_time = SECOND_TARGET_START;
    arm(&fixture.ledger, &late).unwrap();
    activate_record(&fixture, SECOND_RECORD);
    settle_worker_termination(&fixture.settings, &claim).unwrap();
    assert_record(
        &fixture,
        SECOND_RECORD,
        CustodyPublication::Active,
        CustodyState::Settled,
    );
}

#[test]
fn exact_worker_claim_cannot_settle_another_worker() {
    let fixture = Fixture::new();
    arm(&fixture.ledger, &fixture.spec).unwrap();
    set_active(&fixture);
    let claim = begin_worker_termination(&fixture.settings, &fixture.spec.worker).unwrap();
    let mut other = fixture.spec.clone();
    other.invocation_id = uuid::Uuid::new_v4();
    other.worker.pid = OTHER_WORKER_PID;
    arm(&fixture.ledger, &other).unwrap();
    activate_record(&fixture, SECOND_RECORD);
    settle_worker_termination(&fixture.settings, &claim).unwrap();
    assert_record(
        &fixture,
        SECOND_RECORD,
        CustodyPublication::Active,
        CustodyState::Armed,
    );
}

fn activate_record(fixture: &Fixture, index: usize) {
    persistence::update(&fixture.ledger, |ledger| {
        ledger.records[index].publication = CustodyPublication::Active;
        Ok(())
    })
    .unwrap();
}

fn assert_record(
    fixture: &Fixture,
    index: usize,
    publication: CustodyPublication,
    state: CustodyState,
) {
    persistence::update(&fixture.ledger, |ledger| {
        assert_eq!(ledger.records[index].publication, publication);
        assert_eq!(ledger.records[index].state, state);
        Ok(())
    })
    .unwrap();
}
