use crate::{constants::*, fixture::*};
use yolo_hub_agent_protocol::AgentWorkingDirectory;
use yolo_hub_connector_protocol::{ConnectorError, FIRST_PUBLICATION_SEQUENCE};
use yolo_hub_identity::{
    ConnectorGeneration, ConnectorId, ConnectorInstanceId, ExecutionId, RuntimeEpoch,
};

#[test]
fn exact_duplicate_is_acknowledged_without_advancing_cursor_time_or_file_identity() {
    let fixture = Fixture::new();
    let mut registry = fixture.open();
    let enrollment = fixture.register(&mut registry);
    let active = fixture.activate(&mut registry, &enrollment, ConnectorInstanceId::new());
    let original = publication(active, FIRST_PUBLICATION_SEQUENCE);
    let first = registry
        .publish(active, original.clone(), fixture.admission(), NOW)
        .unwrap();
    assert!(!first.duplicate);
    let before = registry.snapshots().unwrap();
    let path = fixture.member(enrollment.connector_id);
    let version = fixture.version(&path);
    let repeated = registry
        .publish(active, original, fixture.admission(), LATER)
        .unwrap();
    assert!(repeated.duplicate);
    assert_eq!(repeated.cursor, first.cursor);
    assert_eq!(registry.snapshots().unwrap(), before);
    assert_eq!(fixture.version(&path), version);
    let next = registry
        .publish(
            active,
            publication(active, FIRST_PUBLICATION_SEQUENCE + STEP),
            fixture.admission(),
            LATER,
        )
        .unwrap();
    assert!(!next.duplicate);
    assert_eq!(next.cursor.sequence(), first.cursor.sequence() + STEP);
}

#[test]
fn changed_duplicate_gap_zero_and_old_sequence_are_rejected_without_advancement() {
    let fixture = Fixture::new();
    let mut registry = fixture.open();
    let enrollment = fixture.register(&mut registry);
    let active = fixture.activate(&mut registry, &enrollment, ConnectorInstanceId::new());
    let original = publication(active, FIRST_PUBLICATION_SEQUENCE);
    registry
        .publish(active, original.clone(), fixture.admission(), NOW)
        .unwrap();
    let path = fixture.member(enrollment.connector_id);
    let version = fixture.version(&path);
    let mut changed = original;
    changed.state.working_directory =
        Some(AgentWorkingDirectory::new(OTHER_DIRECTORY.into()).unwrap());
    assert_eq!(
        registry.publish(active, changed, fixture.admission(), LATER),
        Err(ConnectorError::Sequence)
    );
    for sequence in [0, FIRST_PUBLICATION_SEQUENCE + STEP + STEP, u64::MAX] {
        assert_eq!(
            registry.publish(
                active,
                publication(active, sequence),
                fixture.admission(),
                LATER
            ),
            Err(ConnectorError::Sequence)
        );
    }
    assert_eq!(fixture.version(&path), version);
    registry
        .publish(
            active,
            publication(active, FIRST_PUBLICATION_SEQUENCE + STEP),
            fixture.admission(),
            LATER,
        )
        .unwrap();
    assert_eq!(
        registry.publish(
            active,
            publication(active, FIRST_PUBLICATION_SEQUENCE),
            fixture.admission(),
            LATER
        ),
        Err(ConnectorError::Sequence)
    );
}

#[test]
fn publications_cannot_redirect_identity_generation_or_terminal() {
    let fixture = Fixture::new();
    let mut registry = fixture.open();
    let enrollment = fixture.register(&mut registry);
    let active = fixture.activate(&mut registry, &enrollment, ConnectorInstanceId::new());
    let mutations: &[fn(&mut yolo_hub_connector_protocol::Publication)] = &[
        |p| p.connector_id = ConnectorId::new(),
        |p| p.generation = ConnectorGeneration::new(),
        |p| p.agent.execution_id = ExecutionId::new(),
        |p| p.terminal.runtime_epoch = RuntimeEpoch::new(),
    ];
    for mutate in mutations {
        let mut wrong = publication(active, FIRST_PUBLICATION_SEQUENCE);
        mutate(&mut wrong);
        assert_eq!(
            registry.publish(active, wrong, fixture.admission(), NOW),
            Err(ConnectorError::StaleGeneration)
        );
    }
    assert!(registry.snapshots().unwrap().is_empty());
    registry
        .publish(
            active,
            publication(active, FIRST_PUBLICATION_SEQUENCE),
            fixture.admission(),
            NOW,
        )
        .unwrap();
}
