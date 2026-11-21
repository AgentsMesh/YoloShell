use super::*;

pub(super) fn persist_raw(
    fixture: &Fixture,
    event: HookEnvelope,
) -> (CodexHookIntake, HookEnvelope) {
    let intake = intake(fixture);
    let occurrence = intake
        .persist_occurrence_for_test(unassigned(event))
        .unwrap();
    (intake, occurrence)
}

pub(super) fn enqueue_durable(
    fixture: &Fixture,
    intake: &CodexHookIntake,
    occurrence: HookEnvelope,
) -> HookEnvelope {
    let identity = intake
        .find_exact_occurrence(&occurrence)
        .unwrap()
        .unwrap()
        .durable_identity()
        .unwrap();
    outbox(fixture)
        .enqueue_admitted_for_test(occurrence, Some(&identity), |_, _| Ok(()), |_, _, _| Ok(()))
        .unwrap()
        .0
}

pub(super) fn unassigned(mut event: HookEnvelope) -> HookEnvelope {
    event.event_id = Uuid::nil();
    event.sequence = EventSequence::ZERO;
    event
}

fn intake(fixture: &Fixture) -> CodexHookIntake {
    CodexHookIntake::for_identity(artifacts(fixture).intake_directory(), &fixture.identity).unwrap()
}

pub(super) fn outbox(fixture: &Fixture) -> CodexOutbox {
    CodexOutbox::for_identity(artifacts(fixture).outbox_path(), &fixture.identity).unwrap()
}

fn artifacts(fixture: &Fixture) -> CodexHookArtifacts {
    CodexHookArtifacts::new(
        &fixture.paths,
        fixture.identity.terminal_id,
        fixture.identity.runtime_epoch,
        fixture.identity.invocation_id,
    )
    .unwrap()
}
