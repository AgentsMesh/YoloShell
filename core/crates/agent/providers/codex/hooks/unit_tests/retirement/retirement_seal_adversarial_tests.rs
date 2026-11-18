use super::retirement_test_support::{
    MALFORMED_JSON, RetirementFixture, SealMutation, mutate_seal, replace_same_content,
    replace_with,
};

#[test]
fn conflicting_and_corrupt_seal_replays_fail_closed() {
    let fixture = RetirementFixture::new();
    let writer = fixture.writer();
    let seal = fixture.seal();
    writer.publish_seal(&seal, fixture.admission()).unwrap();

    let foreign = RetirementFixture::new();
    let conflicting = serde_json::to_vec_pretty(&foreign.seal()).unwrap();
    replace_with(&fixture.seal_path(), &conflicting);
    assert!(writer.publish_seal(&seal, fixture.admission()).is_err());
    assert_eq!(std::fs::read(fixture.seal_path()).unwrap(), conflicting);

    replace_with(&fixture.seal_path(), MALFORMED_JSON);
    assert!(writer.publish_seal(&seal, fixture.admission()).is_err());
    assert_eq!(std::fs::read(fixture.seal_path()).unwrap(), MALFORMED_JSON);
}

#[test]
fn exact_seal_authority_rejects_every_path_mutation() {
    for mutation in [
        SealMutation::Deleted,
        SealMutation::DifferentContent,
        SealMutation::SameContentNewIdentity,
    ] {
        let fixture = RetirementFixture::new();
        let writer = fixture.writer();
        let publication = writer
            .publish_seal(&fixture.seal(), fixture.admission())
            .unwrap();
        let artifact = publication.artifact().clone();
        mutate_seal(&fixture, mutation);

        assert!(writer.verify_seal(&artifact, fixture.admission()).is_err());
    }
}

#[test]
fn same_payload_admission_recreation_revokes_retirement_authority() {
    let fixture = RetirementFixture::new();
    let writer = fixture.writer();
    let seal = fixture.seal();
    replace_same_content(fixture.admission_path());
    let replacement = fixture.recovered_admission();

    assert!(fixture.admission().verify().is_err());
    assert!(!fixture.admission().exactly_matches(&replacement));
    replacement.verify().unwrap();
    assert!(writer.publish_seal(&seal, fixture.admission()).is_err());
    assert!(seal.validate_for(&replacement).is_err());
}
