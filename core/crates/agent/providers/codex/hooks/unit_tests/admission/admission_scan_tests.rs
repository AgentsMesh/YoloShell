use super::admission_test_support::Fixture;
use super::{
    CodexAdmissionAuthority, CodexAdmissionRecord, CodexAdmissionStorage, CodexHookArtifacts,
    scan_admissions,
};
use uuid::Uuid;

const FIRST_INVOCATION: u128 = 0x73000000000000000000000000000001;
const SECOND_INVOCATION: u128 = 0x73000000000000000000000000000002;
const FIRST_ROUTE_REVISION: u64 = 1;
const SECOND_ROUTE_REVISION: u64 = 2;
const EMPTY_ORPHAN_INVOCATION: u128 = 0x73000000000000000000000000000003;
const LOCK_ORPHAN_INVOCATION: u128 = 0x73000000000000000000000000000004;
const INTAKE_ORPHAN_INVOCATION: u128 = 0x73000000000000000000000000000005;
const CORRUPT_ORPHAN_INVOCATION: u128 = 0x73000000000000000000000000000006;
const INVALID_ENTRY: &str = "not-an-invocation";
const CORRUPT_ENTRY: &str = "unexpected";
const FIRST_INDEX: usize = 0;
const SECOND_INDEX: usize = 1;
const EXPECTED_ADMISSIONS: usize = 2;
const FIRST_SCAN_INVOCATION: usize = 1;
const OVERFLOW_ENTRY_COUNT: usize = 1;
const EMPTY_RECLAIM_OVERFLOW_COUNT: usize = 1;
const DISPLACED_EMPTY_INVOCATION: &str = "displaced-empty-invocation";

#[test]
fn scan_recovers_exact_admissions_in_stable_route_revision_order() {
    let fixture = Fixture::new();
    let second = publish(&fixture, SECOND_INVOCATION, SECOND_ROUTE_REVISION);
    let first = publish(&fixture, FIRST_INVOCATION, FIRST_ROUTE_REVISION);

    let recovered = scan_admissions(
        &fixture.paths,
        fixture.identity.terminal_id,
        fixture.identity.runtime_epoch,
    )
    .unwrap();

    assert_eq!(recovered.len(), EXPECTED_ADMISSIONS);
    assert!(recovered[FIRST_INDEX].exactly_matches(&first));
    assert!(recovered[SECOND_INDEX].exactly_matches(&second));
}

#[test]
fn scan_rejects_non_invocation_entries() {
    let fixture = Fixture::new();
    let root = runtime_root(&fixture);
    crate::paths::directory::PrivateDirectory::ensure(&root.join(INVALID_ENTRY)).unwrap();
    assert!(scan(&fixture).is_err());
}

#[test]
fn scan_skips_exact_pre_admission_power_loss_residue() {
    let fixture = Fixture::new();
    orphan_artifacts(&fixture, EMPTY_ORPHAN_INVOCATION);
    let locked = orphan_artifacts(&fixture, LOCK_ORPHAN_INVOCATION);
    create_lock(&locked);
    let with_intake = orphan_artifacts(&fixture, INTAKE_ORPHAN_INVOCATION);
    create_lock(&with_intake);
    crate::paths::directory::PrivateDirectory::ensure(&with_intake.intake_directory()).unwrap();

    assert!(scan(&fixture).unwrap().is_empty());
}

#[test]
fn scan_rejects_nonempty_pre_admission_residue() {
    let fixture = Fixture::new();
    let artifacts = orphan_artifacts(&fixture, CORRUPT_ORPHAN_INVOCATION);
    crate::paths::directory::PrivateDirectory::ensure(
        &artifacts.intake_directory().join(CORRUPT_ENTRY),
    )
    .unwrap();
    assert!(scan(&fixture).is_err());
}

#[test]
fn scan_fails_closed_when_the_entry_budget_is_exceeded() {
    let fixture = Fixture::new();
    let final_invocation =
        crate::paths::constants::MAX_PRIVATE_DIRECTORY_ENTRIES + OVERFLOW_ENTRY_COUNT;
    for index in FIRST_SCAN_INVOCATION..=final_invocation {
        publish(
            &fixture,
            u128::try_from(index).unwrap(),
            u64::try_from(index).unwrap(),
        );
    }
    assert!(scan(&fixture).is_err());
}

#[test]
fn scan_reclaims_more_than_the_retained_entry_budget_of_empty_invocations() {
    let fixture = Fixture::new();
    let count =
        crate::paths::constants::MAX_PRIVATE_DIRECTORY_ENTRIES + EMPTY_RECLAIM_OVERFLOW_COUNT;
    for index in FIRST_SCAN_INVOCATION..=count {
        orphan_artifacts(&fixture, u128::try_from(index).unwrap());
    }

    assert!(scan(&fixture).unwrap().is_empty());
    assert_eq!(
        std::fs::read_dir(runtime_root(&fixture)).unwrap().count(),
        0
    );
}

#[test]
fn empty_invocation_replacement_is_never_removed_as_the_retained_identity() {
    let fixture = Fixture::new();
    let artifacts = orphan_artifacts(&fixture, EMPTY_ORPHAN_INVOCATION);
    let directory = artifacts.directory().to_path_buf();
    let displaced = runtime_root(&fixture)
        .parent()
        .unwrap()
        .join(DISPLACED_EMPTY_INVOCATION);
    let directory_for_hook = directory.clone();
    let displaced_for_hook = displaced.clone();
    crate::paths::test_support::set_before_empty_remove_hook(move || {
        std::fs::rename(&directory_for_hook, &displaced_for_hook).unwrap();
        crate::paths::directory::PrivateDirectory::ensure(&directory_for_hook).unwrap();
    });

    assert!(scan(&fixture).unwrap().is_empty());
    assert!(directory.exists());
    assert!(displaced.exists());
}

#[test]
fn scan_rejects_same_payload_admission_replacement_before_validation() {
    let fixture = Fixture::new();
    let authority = publish(&fixture, FIRST_INVOCATION, FIRST_ROUTE_REVISION);
    let path = authority.path().to_path_buf();
    super::admission_scan::set_before_validation_hook(move || {
        super::retirement_test_support::replace_same_content(&path);
    });

    assert!(scan(&fixture).is_err());
}

fn scan(fixture: &Fixture) -> anyhow::Result<Vec<CodexAdmissionAuthority>> {
    scan_admissions(
        &fixture.paths,
        fixture.identity.terminal_id,
        fixture.identity.runtime_epoch,
    )
}

fn runtime_root(fixture: &Fixture) -> std::path::PathBuf {
    CodexHookArtifacts::runtime_directory(
        &fixture.paths,
        fixture.identity.terminal_id,
        fixture.identity.runtime_epoch,
    )
    .unwrap()
}

fn orphan_artifacts(fixture: &Fixture, invocation: u128) -> CodexHookArtifacts {
    let artifacts = CodexHookArtifacts::new(
        &fixture.paths,
        fixture.identity.terminal_id,
        fixture.identity.runtime_epoch,
        Uuid::from_u128(invocation),
    )
    .unwrap();
    crate::paths::directory::PrivateDirectory::ensure(artifacts.directory()).unwrap();
    artifacts
}

fn create_lock(artifacts: &CodexHookArtifacts) {
    drop(
        crate::paths::private_file::PrivateFileEntry::ensure(
            &artifacts.retirement_writer_lock_path(),
        )
        .unwrap()
        .open_rw_or_create(super::constants::LOCK_RECORD_MAX_BYTES)
        .unwrap(),
    );
}

fn publish(fixture: &Fixture, invocation: u128, route_revision: u64) -> CodexAdmissionAuthority {
    let mut request = fixture.request.clone();
    request.identity.invocation_id = Uuid::from_u128(invocation);
    request.identity.hook_token = Uuid::new_v4();
    let artifacts = CodexHookArtifacts::new(
        &fixture.paths,
        request.identity.terminal_id,
        request.identity.runtime_epoch,
        request.identity.invocation_id,
    )
    .unwrap();
    let storage = CodexAdmissionStorage::for_identity(&artifacts, &request.identity).unwrap();
    let record =
        CodexAdmissionRecord::capture_with_route_revision(&request, route_revision).unwrap();
    let publication = storage.publish(&record).unwrap();
    CodexAdmissionAuthority::from_publication(storage, publication).unwrap()
}
