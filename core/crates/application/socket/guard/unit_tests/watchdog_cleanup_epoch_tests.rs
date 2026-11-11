use super::*;

const HISTORICAL_PID: u32 = 41;
const HISTORICAL_START_TIME: u64 = 43;
const NEW_PID: u32 = 47;
const NEW_START_TIME: u64 = 53;
#[test]
fn all_identity_refresh_is_not_mistaken_for_a_new_discovery() {
    let historical = identity(HISTORICAL_PID, HISTORICAL_START_TIME);

    assert!(!has_new_identity(Some(&[historical]), Some(&[historical])));
    assert!(has_new_identity(
        Some(&[historical]),
        Some(&[historical, identity(NEW_PID, NEW_START_TIME)])
    ));
}

#[test]
fn uncertain_identity_observations_cannot_trigger_a_handoff() {
    let historical = identity(HISTORICAL_PID, HISTORICAL_START_TIME);

    assert!(!has_new_identity(None, Some(&[historical])));
    assert!(!has_new_identity(Some(&[historical]), None));
}

fn identity(pid: u32, start_time: u64) -> ProcessIdentity {
    ProcessIdentity { pid, start_time }
}
