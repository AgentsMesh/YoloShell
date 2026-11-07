use super::retirement_test_support::{
    RetirementFixture, SealMutation, mutate_seal, replace_snapshot_after_retirement_install,
};

#[test]
fn retirement_requires_the_exact_published_seal() {
    for mutation in [
        SealMutation::Deleted,
        SealMutation::DifferentContent,
        SealMutation::SameContentNewIdentity,
    ] {
        let fixture = RetirementFixture::new();
        let writer = fixture.writer();
        let seal = fixture.seal();
        let seal_publication = writer.publish_seal(&seal, fixture.admission()).unwrap();
        let terminal = fixture.terminal(&writer, seal_publication.artifact());
        mutate_seal(&fixture, mutation);

        assert!(
            writer
                .publish_retirement(
                    &terminal.record,
                    seal_publication.artifact(),
                    &terminal.snapshot,
                    fixture.admission(),
                    &terminal.proof,
                    &terminal.intake,
                    &terminal.outbox,
                )
                .is_err()
        );
        assert!(!fixture.retirement_path().exists());
    }
}

#[test]
fn terminal_snapshot_failure_after_install_rolls_back_tombstone() {
    let fixture = RetirementFixture::new();
    let writer = fixture.writer();
    let seal = fixture.seal();
    let seal_publication = writer.publish_seal(&seal, fixture.admission()).unwrap();
    let terminal = fixture.terminal(&writer, seal_publication.artifact());
    replace_snapshot_after_retirement_install(&fixture.snapshot_path(), &fixture.retirement_path());

    assert!(
        writer
            .publish_retirement(
                &terminal.record,
                seal_publication.artifact(),
                &terminal.snapshot,
                fixture.admission(),
                &terminal.proof,
                &terminal.intake,
                &terminal.outbox,
            )
            .is_err()
    );
    assert!(!fixture.retirement_path().exists());
}

#[test]
fn preflight_snapshot_mismatch_never_creates_a_tombstone() {
    let fixture = RetirementFixture::new();
    let writer = fixture.writer();
    let seal = fixture.seal();
    let seal_publication = writer.publish_seal(&seal, fixture.admission()).unwrap();
    let terminal = fixture.terminal(&writer, seal_publication.artifact());
    let mutated = fixture.alternate_terminal_snapshot();

    assert!(
        writer
            .publish_retirement(
                &terminal.record,
                seal_publication.artifact(),
                &mutated,
                fixture.admission(),
                &terminal.proof,
                &terminal.intake,
                &terminal.outbox,
            )
            .is_err()
    );
    assert!(!fixture.retirement_path().exists());
}
