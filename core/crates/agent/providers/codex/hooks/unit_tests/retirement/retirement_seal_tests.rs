use super::retirement_test_support::RetirementFixture;
use uuid::Uuid;

#[test]
fn seal_first_publication_and_exact_replay_are_stable() {
    let fixture = RetirementFixture::new();
    let writer = fixture.writer();
    let seal = fixture.seal();

    let first = writer.publish_seal(&seal, fixture.admission()).unwrap();
    assert!(!first.is_replay());
    let artifact = first.artifact().clone();

    let replay = writer.publish_seal(&seal, fixture.admission()).unwrap();
    assert!(replay.is_replay());
    assert_eq!(replay.artifact(), &artifact);
    writer
        .verify_seal(replay.artifact(), fixture.admission())
        .unwrap();
}

#[test]
fn sealed_invocation_rejects_late_producers_without_new_occurrences() {
    let fixture = RetirementFixture::new();
    let intake = fixture.intake();
    intake.persist(fixture.envelope()).unwrap();
    let before = intake.scan().unwrap().occurrences.len();
    let writer = fixture.writer();
    writer
        .publish_seal(&fixture.seal(), fixture.admission())
        .unwrap();
    drop(writer);

    let mut late = fixture.envelope();
    late.event_id = Uuid::new_v4();
    assert!(intake.persist(late).is_err());
    assert_eq!(intake.scan().unwrap().occurrences.len(), before);
}

#[test]
fn seal_candidate_is_bound_to_the_admitted_identity() {
    let fixture = RetirementFixture::new();
    let foreign = RetirementFixture::new();

    assert!(fixture.seal().validate_for(foreign.admission()).is_err());
    assert!(
        fixture
            .writer()
            .publish_seal(&foreign.seal(), fixture.admission())
            .is_err()
    );
}
