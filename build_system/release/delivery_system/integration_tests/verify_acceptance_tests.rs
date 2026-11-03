use super::developer_release_acceptance_support::GOLDEN_RECEIPT_EXPIRED_NOW;
use super::verify_acceptance_support::{
    ACCEPTANCE_NOW, CandidateBindingDrift, VerifyAcceptanceFixture,
};
use yolo_release_verification::acceptance_receipt_candidate::verify_receipt_for_candidate_at_with_test_probe;

#[test]
fn exact_receipt_verifies_without_creating_delivery_state() {
    let fixture = VerifyAcceptanceFixture::new();
    let expected = fixture.opened_receipt().record().clone();

    assert_eq!(fixture.verify_at(ACCEPTANCE_NOW).unwrap(), expected);
    fixture.assert_no_output();
}

#[test]
fn expired_receipt_is_rejected_without_creating_delivery_state() {
    let fixture = VerifyAcceptanceFixture::new();

    assert!(fixture.verify_at(GOLDEN_RECEIPT_EXPIRED_NOW).is_err());
    fixture.assert_no_output();
}

#[test]
fn every_candidate_binding_field_is_enforced() {
    for drift in [
        CandidateBindingDrift::SourceRevision,
        CandidateBindingDrift::ArtifactDigest,
        CandidateBindingDrift::ProvenanceDigest,
    ] {
        let fixture = VerifyAcceptanceFixture::new();
        fixture.drift_binding(drift);

        assert!(fixture.verify_at(ACCEPTANCE_NOW).is_err());
        fixture.assert_no_output();
    }
}

#[test]
fn receipt_path_replacement_at_final_fence_is_rejected() {
    let fixture = VerifyAcceptanceFixture::new();
    let candidate = fixture.opened_candidate();
    let receipt = fixture.opened_receipt();

    assert!(
        verify_receipt_for_candidate_at_with_test_probe(
            &receipt,
            &candidate,
            ACCEPTANCE_NOW,
            || fixture.replace_receipt_path(),
        )
        .is_err()
    );
    fixture.assert_no_output();
}

#[test]
fn candidate_path_replacement_at_final_fence_is_rejected() {
    let fixture = VerifyAcceptanceFixture::new();
    let candidate = fixture.opened_candidate();
    let receipt = fixture.opened_receipt();

    assert!(
        verify_receipt_for_candidate_at_with_test_probe(
            &receipt,
            &candidate,
            ACCEPTANCE_NOW,
            || fixture.replace_candidate_path(),
        )
        .is_err()
    );
    fixture.assert_no_output();
}

#[test]
fn candidate_file_replacement_at_final_fence_is_rejected() {
    let fixture = VerifyAcceptanceFixture::new();
    let candidate = fixture.opened_candidate();
    let receipt = fixture.opened_receipt();

    assert!(
        verify_receipt_for_candidate_at_with_test_probe(
            &receipt,
            &candidate,
            ACCEPTANCE_NOW,
            || fixture.replace_artifact_path(),
        )
        .is_err()
    );
    fixture.assert_no_output();
}
