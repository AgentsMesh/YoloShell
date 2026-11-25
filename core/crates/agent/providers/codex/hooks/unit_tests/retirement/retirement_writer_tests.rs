use super::retirement_test_support::RetirementFixture;

#[test]
fn retirement_first_publication_and_exact_replay_are_stable() {
    let fixture = RetirementFixture::new();
    let writer = fixture.writer();
    let seal = fixture.seal();
    let seal_publication = writer.publish_seal(&seal, fixture.admission()).unwrap();
    let terminal = fixture.terminal(&writer, seal_publication.artifact());

    let first = writer
        .publish_retirement(
            &terminal.record,
            seal_publication.artifact(),
            &terminal.snapshot,
            fixture.admission(),
            &terminal.proof,
            &terminal.intake,
            &terminal.outbox,
        )
        .unwrap();
    assert!(!first.is_replay());
    let artifact = first.artifact().clone();

    let replay = writer
        .publish_retirement(
            &terminal.record,
            seal_publication.artifact(),
            &terminal.snapshot,
            fixture.admission(),
            &terminal.proof,
            &terminal.intake,
            &terminal.outbox,
        )
        .unwrap();
    assert!(replay.is_replay());
    assert_eq!(replay.artifact(), &artifact);
    writer
        .verify_retirement(
            replay.artifact(),
            seal_publication.artifact(),
            &terminal.snapshot,
            fixture.admission(),
            &terminal.proof,
        )
        .unwrap();
}

#[test]
fn retirement_record_rejects_another_terminal_snapshot() {
    let fixture = RetirementFixture::new();
    let writer = fixture.writer();
    let seal = fixture.seal();
    let seal = writer.publish_seal(&seal, fixture.admission()).unwrap();
    let terminal = fixture.terminal(&writer, seal.artifact());
    let mutated = fixture.alternate_terminal_snapshot();

    assert!(
        terminal
            .record
            .validate_for(
                &mutated,
                fixture.admission(),
                seal.artifact(),
                &terminal.proof,
            )
            .is_err()
    );
    terminal
        .record
        .validate_for(
            &terminal.snapshot,
            fixture.admission(),
            seal.artifact(),
            &terminal.proof,
        )
        .unwrap();
}
