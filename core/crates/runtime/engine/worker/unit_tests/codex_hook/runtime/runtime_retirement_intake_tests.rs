use super::runtime_intake_test_support::RoutedFixture;
use super::runtime_retirement_intake_test_support::{
    RetirementResources, assigned, persisted_occurrence,
};
use crate::worker::codex_hook::runtime::{CodexIngressEvidence, CodexRetirementIntakeAuthority};

#[test]
fn exact_retirement_capabilities_authorize_the_durable_occurrence() {
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
    let event = assigned(&occurrence);
    let evidence = CodexIngressEvidence::VerifiedRetirementOccurrence(proof);

    fixture.runtime.admit_event(&event, &evidence).unwrap();
}

#[test]
fn a_foreign_runtime_cannot_borrow_the_retirement_capabilities() {
    let fixture = RoutedFixture::new();
    let resources = RetirementResources::new(&fixture);
    let (foreign_runtime, _) = fixture.restart_runtime();

    let result = CodexRetirementIntakeAuthority::new(
        &foreign_runtime,
        resources.permit(),
        resources.writer(),
        resources.seal(),
    );

    assert!(result.is_err());
}

#[test]
fn a_foreign_permit_cannot_borrow_the_retirement_capabilities() {
    let fixture = RoutedFixture::new();
    let resources = RetirementResources::new(&fixture);
    let foreign = RoutedFixture::new();
    let foreign_resources = RetirementResources::new(&foreign);

    let result = CodexRetirementIntakeAuthority::new(
        &fixture.runtime,
        foreign_resources.permit(),
        resources.writer(),
        resources.seal(),
    );

    assert!(result.is_err());
}
