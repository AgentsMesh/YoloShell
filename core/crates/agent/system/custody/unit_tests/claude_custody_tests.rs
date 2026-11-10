use super::*;

const TARGET_PID: u32 = 910_001;
const TARGET_START: u64 = 910_002;
const WORKER_PID: u32 = 910_003;
const WORKER_START: u64 = 910_004;
const PRIMARY_PID: u32 = 910_005;
const PRIMARY_START: u64 = 910_006;
const STANDBY_PID: u32 = 910_007;
const STANDBY_START: u64 = 910_008;
const TEST_TTY_PATH: &str = "/dev/null";
const PREVIOUS_BOOT_ID: &str = "test-previous-boot";
const RECORDS_FIELD: &str = "records";
const PUBLICATION_FIELD: &str = "publication";
const PRIMARY_GUARDIAN_INDEX: usize = 0;
const FIRST_RECORD_INDEX: usize = 0;

#[test]
fn unpublished_ownerless_custody_is_retired() {
    let fixture = Fixture::new();
    arm(&fixture.ledger, &fixture.spec).unwrap();
    admit(&fixture.settings).unwrap();
    admit(&fixture.settings).unwrap();
}

#[test]
fn published_ownerless_custody_becomes_indeterminate() {
    let fixture = Fixture::new();
    arm(&fixture.ledger, &fixture.spec).unwrap();
    set_active(&fixture);
    let error = admit(&fixture.settings).unwrap_err();
    assert!(error.to_string().contains("Active Indeterminate"));
    assert!(admit(&fixture.settings).is_err());
}

#[test]
fn unpublished_failed_custody_is_retired_after_every_owner_exits() {
    for transition in [mark_indeterminate, mark_containing] {
        let fixture = Fixture::new();
        arm(&fixture.ledger, &fixture.spec).unwrap();
        transition(&fixture.ledger, &fixture.spec).unwrap();
        admit(&fixture.settings).unwrap();
    }
}

#[test]
fn later_guardian_closure_proof_converges_indeterminate_custody() {
    let fixture = Fixture::new();
    arm(&fixture.ledger, &fixture.spec).unwrap();
    mark_indeterminate(&fixture.ledger, &fixture.spec).unwrap();
    mark_containing(&fixture.ledger, &fixture.spec).unwrap();
    mark_settled(&fixture.ledger, &fixture.spec).unwrap();
    admit(&fixture.settings).unwrap();
}

#[test]
fn failed_guardian_race_cannot_block_peer_closure_proof() {
    let fixture = Fixture::new();
    arm(&fixture.ledger, &fixture.spec).unwrap();
    let barrier = std::sync::Barrier::new(types::GUARDIAN_COUNT);
    std::thread::scope(|scope| {
        let failed = scope.spawn(|| {
            barrier.wait();
            let _ = mark_indeterminate(&fixture.ledger, &fixture.spec);
        });
        let survivor = scope.spawn(|| {
            barrier.wait();
            mark_containing(&fixture.ledger, &fixture.spec).unwrap();
            mark_settled(&fixture.ledger, &fixture.spec).unwrap();
        });
        failed.join().unwrap();
        survivor.join().unwrap();
    });
    admit(&fixture.settings).unwrap();
}

#[test]
fn proven_settlement_is_retired_during_admission() {
    let fixture = Fixture::new();
    arm(&fixture.ledger, &fixture.spec).unwrap();
    mark_settled(&fixture.ledger, &fixture.spec).unwrap();
    admit(&fixture.settings).unwrap();
    admit(&fixture.settings).unwrap();
}

#[test]
fn custody_from_a_previous_boot_is_proven_settled_during_admission() {
    let mut fixture = Fixture::new();
    fixture.spec.target.boot_id = PREVIOUS_BOOT_ID.into();
    fixture.spec.worker.boot_id = PREVIOUS_BOOT_ID.into();
    for guardian in &mut fixture.spec.guardians {
        guardian.boot_id = PREVIOUS_BOOT_ID.into();
    }
    arm(&fixture.ledger, &fixture.spec).unwrap();
    admit(&fixture.settings).unwrap();
    admit(&fixture.settings).unwrap();
}

#[test]
fn published_incomplete_previous_boot_evidence_fails_closed() {
    for unresolved_boot in [
        crate::process::boot_id(),
        crate::process::unknown_boot_id_for_test(),
    ] {
        let mut fixture = Fixture::new();
        fixture.spec.target.boot_id = PREVIOUS_BOOT_ID.into();
        fixture.spec.worker.boot_id = PREVIOUS_BOOT_ID.into();
        for guardian in &mut fixture.spec.guardians {
            guardian.boot_id = PREVIOUS_BOOT_ID.into();
        }
        fixture.spec.guardians[PRIMARY_GUARDIAN_INDEX].boot_id = unresolved_boot;
        arm(&fixture.ledger, &fixture.spec).unwrap();
        set_active(&fixture);
        assert!(admit(&fixture.settings).is_err());
    }
}

#[test]
fn missing_publication_is_rejected_without_rewriting_custody() {
    let fixture = Fixture::new();
    arm(&fixture.ledger, &fixture.spec).unwrap();
    let mut value: serde_json::Value = crate::paths::read_json(&fixture.ledger).unwrap();
    value[RECORDS_FIELD][FIRST_RECORD_INDEX]
        .as_object_mut()
        .unwrap()
        .remove(PUBLICATION_FIELD);
    crate::paths::atomic_write_json(&fixture.ledger, &value).unwrap();
    let before = std::fs::read(&fixture.ledger).unwrap();
    assert!(serde_json::from_value::<types::CustodyLedger>(value).is_err());
    assert!(admit(&fixture.settings).is_err());
    assert_eq!(std::fs::read(&fixture.ledger).unwrap(), before);
}

struct Fixture {
    _directory: tempfile::TempDir,
    settings: std::path::PathBuf,
    ledger: std::path::PathBuf,
    spec: CustodySpec,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let settings = directory
            .path()
            .join(crate::claude_settings_artifact::DIRECTORY_NAME);
        crate::paths::directory::PrivateDirectory::ensure(&settings).unwrap();
        let ledger = path(&settings).unwrap();
        let authority = crate::claude_hooks::ClaudeCustodyAuthority {
            worker: crate::claude_hooks::ClaudeProcessIdentity {
                pid: WORKER_PID,
                start_time: WORKER_START,
            },
            worker_boot_id: crate::process::boot_id(),
            tty_path: TEST_TTY_PATH.into(),
            pty_capability: uuid::Uuid::new_v4().to_string(),
            handshake_deadline_ns: None,
        };
        let spec = spec(
            uuid::Uuid::new_v4(),
            identity(TARGET_PID, TARGET_START),
            &authority,
            identity(PRIMARY_PID, PRIMARY_START),
            identity(STANDBY_PID, STANDBY_START),
        )
        .unwrap();
        Self {
            _directory: directory,
            settings,
            ledger,
            spec,
        }
    }
}

fn identity(pid: u32, start_time: u64) -> crate::process::ProcessIdentity {
    crate::process::ProcessIdentity { pid, start_time }
}

fn set_active(fixture: &Fixture) {
    persistence::update(&fixture.ledger, |ledger| {
        ledger.records[FIRST_RECORD_INDEX].publication = types::CustodyPublication::Active;
        Ok(())
    })
    .unwrap();
}

#[path = "claude_custody_worker_termination_tests.rs"]
mod worker_termination_tests;
