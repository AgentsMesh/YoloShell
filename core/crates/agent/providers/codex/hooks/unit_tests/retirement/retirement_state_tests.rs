use super::retirement_test_support::RetirementFixture;
use crate::codex_hooks::CodexRetirementState;

const EXPECTED_RETIRED_STATE: &str = "expected retired Codex retirement state";
const EXPECTED_SEALED_STATE: &str = "expected sealed Codex retirement state";

#[test]
fn cold_read_reports_an_exact_unsealed_state() {
    let fixture = RetirementFixture::new();

    let state = fixture.writer().read_state(fixture.admission()).unwrap();

    assert!(matches!(state, CodexRetirementState::Unsealed));
}

#[test]
fn cold_read_reports_exact_seal_when_snapshot_is_missing() {
    let fixture = RetirementFixture::new();
    let writer = fixture.writer();
    let publication = writer
        .publish_seal(&fixture.seal(), fixture.admission())
        .unwrap();
    let expected = publication.artifact().clone();
    assert!(!fixture.snapshot_path().exists());

    let state = writer.read_state(fixture.admission()).unwrap();

    let CodexRetirementState::Sealed(actual) = state else {
        panic!("{EXPECTED_SEALED_STATE}");
    };
    assert_eq!(actual, expected);
}

#[test]
fn cold_read_reports_exact_retirement_artifacts() {
    let fixture = RetirementFixture::new();
    let writer = fixture.writer();
    let seal = writer
        .publish_seal(&fixture.seal(), fixture.admission())
        .unwrap();
    let expected_seal = seal.artifact().clone();
    let terminal = fixture.terminal(&writer, seal.artifact());
    let retirement = writer
        .publish_retirement(
            &terminal.record,
            seal.artifact(),
            &terminal.snapshot,
            fixture.admission(),
            &terminal.proof,
            &terminal.intake,
            &terminal.outbox,
        )
        .unwrap();
    let expected_retirement = retirement.artifact().clone();
    drop(terminal.proof);

    let state = writer.read_state(fixture.admission()).unwrap();

    let CodexRetirementState::Retired(actual) = state else {
        panic!("{EXPECTED_RETIRED_STATE}");
    };
    assert_eq!(actual.seal(), &expected_seal);
    assert_eq!(actual.retirement(), &expected_retirement);
}

#[test]
fn cold_read_adopts_unanchored_exact_marker_replacements() {
    let sealed = RetirementFixture::new();
    let writer = sealed.writer();
    let publication = writer
        .publish_seal(&sealed.seal(), sealed.admission())
        .unwrap();
    let original_seal = publication.artifact().file_identity();
    super::retirement_test_support::replace_same_content(&sealed.seal_path());
    let CodexRetirementState::Sealed(replaced) = writer.read_state(sealed.admission()).unwrap()
    else {
        panic!("{EXPECTED_SEALED_STATE}");
    };
    assert_ne!(replaced.file_identity(), original_seal);

    let retired = RetirementFixture::new();
    let writer = retired.writer();
    let seal = writer
        .publish_seal(&retired.seal(), retired.admission())
        .unwrap();
    let terminal = retired.terminal(&writer, seal.artifact());
    let publication = writer
        .publish_retirement(
            &terminal.record,
            seal.artifact(),
            &terminal.snapshot,
            retired.admission(),
            &terminal.proof,
            &terminal.intake,
            &terminal.outbox,
        )
        .unwrap();
    let original_record = publication.artifact().file_identity();
    drop(terminal.proof);
    super::retirement_test_support::replace_same_content(&retired.retirement_path());
    let CodexRetirementState::Retired(replaced) = writer.read_state(retired.admission()).unwrap()
    else {
        panic!("{EXPECTED_RETIRED_STATE}");
    };
    assert_ne!(replaced.retirement().file_identity(), original_record);
}
