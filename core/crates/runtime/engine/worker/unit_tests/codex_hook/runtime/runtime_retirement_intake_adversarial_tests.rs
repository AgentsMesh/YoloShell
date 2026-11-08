use super::runtime_intake_test_support::RoutedFixture;
use super::runtime_retirement_intake_test_support::{
    ForeignLineage, RetirementResources, assigned, intake_directory, persisted_occurrence,
    replace_occurrence,
};
use crate::worker::codex_hook::runtime::CodexRetirementIntakeAuthority;

#[test]
fn a_foreign_seal_cannot_authorize_retirement_intake() {
    let fixture = RoutedFixture::new();
    let resources = RetirementResources::new(&fixture);
    let foreign = RoutedFixture::new();
    let foreign_resources = RetirementResources::new(&foreign);

    let result = CodexRetirementIntakeAuthority::new(
        &fixture.runtime,
        resources.permit(),
        resources.writer(),
        foreign_resources.seal(),
    );

    assert!(result.is_err());
}

#[test]
fn same_identity_in_a_foreign_invocation_lineage_is_rejected() {
    let fixture = RoutedFixture::new();
    let resources = RetirementResources::new(&fixture);
    let foreign = ForeignLineage::new(&fixture);

    let result = CodexRetirementIntakeAuthority::new(
        &fixture.runtime,
        resources.permit(),
        foreign.writer(),
        foreign.seal(),
    );

    assert!(result.is_err());
}

#[test]
fn replaced_occurrence_is_rejected_before_retirement_authorization() {
    let fixture = RoutedFixture::new();
    let (intake, occurrence) = persisted_occurrence(&fixture);
    let resources = RetirementResources::new(&fixture);
    replace_occurrence(&intake_directory(&fixture));
    let authority = CodexRetirementIntakeAuthority::new(
        &fixture.runtime,
        resources.permit(),
        resources.writer(),
        resources.seal(),
    )
    .unwrap();

    assert!(
        authority
            .authorize(&fixture.runtime, &intake, &occurrence)
            .is_err()
    );
}

#[test]
fn replacement_after_authorization_cannot_be_completed_as_the_old_occurrence() {
    let fixture = RoutedFixture::new();
    let (intake, occurrence) = persisted_occurrence(&fixture);
    let resources = RetirementResources::new(&fixture);
    let authority = CodexRetirementIntakeAuthority::new(
        &fixture.runtime,
        resources.permit(),
        resources.writer(),
        resources.seal(),
    )
    .unwrap();
    let proof = authority
        .authorize(&fixture.runtime, &intake, &occurrence)
        .unwrap();
    replace_occurrence(&intake_directory(&fixture));

    proof
        .verify(&fixture.runtime, &assigned(&occurrence))
        .unwrap();
    assert!(intake.complete(&occurrence).is_err());
    assert_eq!(intake.scan().unwrap().occurrences.len(), 1);
}

#[test]
fn replaced_admission_invalidates_an_authorized_retirement_occurrence() {
    let fixture = RoutedFixture::new();
    let (intake, occurrence) = persisted_occurrence(&fixture);
    let resources = RetirementResources::new(&fixture);
    let authority = CodexRetirementIntakeAuthority::new(
        &fixture.runtime,
        resources.permit(),
        resources.writer(),
        resources.seal(),
    )
    .unwrap();
    let proof = authority
        .authorize(&fixture.runtime, &intake, &occurrence)
        .unwrap();
    fixture.replace_admission_same_payload();

    assert!(
        proof
            .verify(&fixture.runtime, &assigned(&occurrence))
            .is_err()
    );
}
