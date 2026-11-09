use super::recovery_test_support::RecoveryFixture;
use super::runtime_retirement_intake_test_support::replace_same_contents;
use crate::codex_hooks::CodexAdmissionAuthority;
use crate::process::ProcessIdentityObservation;
use std::os::unix::fs::PermissionsExt;

const OUTBOX_REPLACEMENT: &str = "recovery-outbox-replacement";
const SEAL_REPLACEMENT: &str = "recovery-seal-replacement";
const SNAPSHOT_REPLACEMENT: &str = "recovery-snapshot-replacement";
const NO_ROUTES: usize = 0;

#[test]
fn live_recovery_rejects_a_seal_created_during_route_install() {
    let fixture = RecoveryFixture::new();
    let (path, contents) = prepared_seal(&fixture.admission);
    super::super::runtime::set_before_initial_publish_hook(move || {
        install_prepared_seal(&path, &contents);
    });

    assert_live_recovery_rejected(&fixture);
}

#[test]
fn live_recovery_rolls_back_when_a_seal_appears_after_install() {
    let fixture = RecoveryFixture::new();
    let (path, contents) = prepared_seal(&fixture.admission);
    super::super::recovery_test_hook::set_after_route_install(move || {
        install_prepared_seal(&path, &contents);
    });

    assert_live_recovery_rejected(&fixture);
}

#[test]
fn round_validation_rolls_back_every_live_route_when_one_is_sealed() {
    let fixture = RecoveryFixture::new();
    let additional = fixture.additional_admission();
    let target = last_admission(&fixture.admission, &additional);
    let (path, contents) = prepared_seal(target);
    super::super::recovery_test_hook::set_before_round_accept(move || {
        install_prepared_seal(&path, &contents);
    });

    assert_live_recovery_rejected(&fixture);
}

#[test]
fn sealed_recovery_rejects_same_payload_seal_replacement() {
    let fixture = RecoveryFixture::new();
    fixture.publish_seal();
    let path = fixture.seal_path();
    super::super::runtime::set_before_initial_publish_hook(move || {
        replace_same_contents(&path, SEAL_REPLACEMENT);
    });

    assert_recovery_rejected(&fixture);
}

#[test]
fn sealed_recovery_rolls_back_route_when_seal_changes_after_install() {
    let fixture = RecoveryFixture::new();
    fixture.publish_seal();
    let path = fixture.seal_path();
    super::super::recovery_test_hook::set_after_route_install(move || {
        replace_same_contents(&path, SEAL_REPLACEMENT);
    });

    assert_recovery_rejected(&fixture);
}

#[test]
fn retired_recovery_rejects_same_payload_snapshot_replacement() {
    let fixture = retired_fixture();
    replace_before_final_validation(fixture.snapshot_path(), SNAPSHOT_REPLACEMENT);

    assert_recovery_rejected(&fixture);
}

#[test]
fn retired_recovery_rejects_same_payload_outbox_replacement() {
    let fixture = retired_fixture_with_start();
    replace_before_final_validation(fixture.outbox_path(), OUTBOX_REPLACEMENT);

    assert_recovery_rejected(&fixture);
}

#[test]
fn retired_recovery_rejects_intake_inserted_after_terminal_proof() {
    let fixture = RecoveryFixture::new();
    fixture.persist_start();
    let (path, contents) = only_occurrence(&fixture);
    retire(&fixture);
    super::super::recovery_retired::set_before_final_validation_hook(move || {
        std::fs::write(&path, contents).unwrap();
        std::fs::set_permissions(
            &path,
            std::fs::Permissions::from_mode(crate::paths::constants::PRIVATE_FILE_MODE),
        )
        .unwrap();
    });

    assert_recovery_rejected(&fixture);
}

fn retired_fixture() -> RecoveryFixture {
    let fixture = RecoveryFixture::new();
    retire(&fixture);
    fixture
}

fn retired_fixture_with_start() -> RecoveryFixture {
    let fixture = RecoveryFixture::new();
    fixture.persist_start();
    retire(&fixture);
    fixture
}

fn retire(fixture: &RecoveryFixture) {
    let (_, result) = fixture.try_recover(ProcessIdentityObservation::Gone);
    result.unwrap();
}

fn replace_before_final_validation(path: std::path::PathBuf, replacement: &'static str) {
    super::super::recovery_retired::set_before_final_validation_hook(move || {
        replace_same_contents(&path, replacement);
    });
}

fn only_occurrence(fixture: &RecoveryFixture) -> (std::path::PathBuf, Vec<u8>) {
    let mut entries = std::fs::read_dir(fixture.intake_directory())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    assert_eq!(entries.len(), 1);
    let path = entries.pop().unwrap();
    let contents = std::fs::read(&path).unwrap();
    (path, contents)
}

fn last_admission<'a>(
    first: &'a CodexAdmissionAuthority,
    second: &'a CodexAdmissionAuthority,
) -> &'a CodexAdmissionAuthority {
    if first.record().request().identity.invocation_id
        > second.record().request().identity.invocation_id
    {
        first
    } else {
        second
    }
}

fn prepared_seal(admission: &CodexAdmissionAuthority) -> (std::path::PathBuf, Vec<u8>) {
    let writer = admission.acquire_retirement_writer_fence().unwrap();
    let seal = super::runtime_retirement_intake_test_support::capture_seal(admission);
    writer.publish_seal(&seal, admission).unwrap();
    let path = admission.retirement_seal_path_for_test();
    let contents = std::fs::read(&path).unwrap();
    std::fs::remove_file(&path).unwrap();
    (path, contents)
}

fn install_prepared_seal(path: &std::path::Path, contents: &[u8]) {
    std::fs::write(path, contents).unwrap();
    std::fs::set_permissions(
        path,
        std::fs::Permissions::from_mode(crate::paths::constants::PRIVATE_FILE_MODE),
    )
    .unwrap();
}

fn assert_recovery_rejected(fixture: &RecoveryFixture) {
    let (router, result) = fixture.try_recover(ProcessIdentityObservation::Unknown);
    assert!(result.is_err());
    assert_eq!(router.len().unwrap(), NO_ROUTES);
}

fn assert_live_recovery_rejected(fixture: &RecoveryFixture) {
    let (router, result) = fixture.try_recover(ProcessIdentityObservation::Matching);
    assert!(result.is_err());
    assert_eq!(router.len().unwrap(), NO_ROUTES);
}
