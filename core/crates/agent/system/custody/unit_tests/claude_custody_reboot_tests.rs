use super::types::{
    CustodyPublication, CustodyRecord, CustodySpec, CustodyState, DurableProcessIdentity,
};
use super::*;

const CURRENT_BOOT_ID: &str = "custody-current-boot";
const PREVIOUS_BOOT_ID: &str = "custody-previous-boot";
const HISTORICAL_BOOT_ID: &str = "custody-historical-boot";
const UNKNOWN_BOOT_ID: &str = "unknown:test-custody-boot";
const BASE_PID: u32 = 71;
const BASE_START_TIME: u64 = 81;
const IDENTITY_INCREMENT: u32 = 1;
const FIRST_RECORD_INDEX: usize = 0;
const SECOND_RECORD_INDEX: usize = 1;

#[test]
fn reboot_rewrite_updates_current_and_preserves_historical_custody() {
    let directory = tempfile::tempdir().unwrap();
    let ledger_path = directory.path().join(LEDGER_FILE);
    append_record(&ledger_path, spec(CURRENT_BOOT_ID, BASE_PID));
    append_record(
        &ledger_path,
        spec(HISTORICAL_BOOT_ID, BASE_PID + IDENTITY_INCREMENT * 4),
    );

    persist_previous_boot_for_test(directory.path(), CURRENT_BOOT_ID, PREVIOUS_BOOT_ID).unwrap();
    persist_previous_boot_for_test(directory.path(), CURRENT_BOOT_ID, PREVIOUS_BOOT_ID).unwrap();

    let ledger: super::types::CustodyLedger = crate::paths::read_json(&ledger_path).unwrap();
    assert_boot(&ledger.records[FIRST_RECORD_INDEX].spec, PREVIOUS_BOOT_ID);
    assert_boot(
        &ledger.records[SECOND_RECORD_INDEX].spec,
        HISTORICAL_BOOT_ID,
    );
}

#[test]
fn unknown_boot_fails_without_partially_rewriting_the_ledger() {
    let directory = tempfile::tempdir().unwrap();
    let ledger_path = directory.path().join(LEDGER_FILE);
    append_record(&ledger_path, spec(CURRENT_BOOT_ID, BASE_PID));
    append_record(
        &ledger_path,
        spec(UNKNOWN_BOOT_ID, BASE_PID + IDENTITY_INCREMENT * 4),
    );
    let before: super::types::CustodyLedger = crate::paths::read_json(&ledger_path).unwrap();

    assert!(
        persist_previous_boot_for_test(directory.path(), CURRENT_BOOT_ID, PREVIOUS_BOOT_ID)
            .is_err()
    );

    let after: super::types::CustodyLedger = crate::paths::read_json(&ledger_path).unwrap();
    assert_eq!(after, before);
}

fn append_record(path: &Path, spec: CustodySpec) {
    persistence::update(path, |ledger| {
        ledger.records.push(CustodyRecord {
            spec,
            state: CustodyState::Settled,
            publication: CustodyPublication::Active,
        });
        Ok(())
    })
    .unwrap();
}

fn spec(boot_id: &str, first_pid: u32) -> CustodySpec {
    let identity = |offset: u32| DurableProcessIdentity {
        boot_id: boot_id.to_owned(),
        pid: first_pid + offset,
        start_time: BASE_START_TIME + u64::from(first_pid + offset),
    };
    CustodySpec {
        invocation_id: uuid::Uuid::new_v4(),
        target: identity(0),
        worker: identity(IDENTITY_INCREMENT),
        guardians: [
            identity(IDENTITY_INCREMENT * 2),
            identity(IDENTITY_INCREMENT * 3),
        ],
    }
}

fn assert_boot(spec: &CustodySpec, expected: &str) {
    assert_eq!(spec.target.boot_id, expected);
    assert_eq!(spec.worker.boot_id, expected);
    assert!(
        spec.guardians
            .iter()
            .all(|identity| identity.boot_id == expected)
    );
}
