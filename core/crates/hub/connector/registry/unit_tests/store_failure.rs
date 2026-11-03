use super::{constants::*, fixture::*};
use yolo_hub_agent_protocol::{
    AgentCapabilities, Freshness, ObservationCursor, Reachability, RecoveryStatus,
};
use yolo_hub_connector_protocol::{
    ConnectorError, FIRST_PUBLICATION_SEQUENCE, MAX_PUBLICATION_SEQUENCE,
};
use yolo_hub_identity::ConnectorInstanceId;

#[test]
fn uncertain_commit_clears_live_authority_and_retains_only_stale_previously_validated_observations()
{
    let fixture = Fixture::new();
    let mut registry = fixture.open();
    let enrollment = fixture.register(&mut registry);
    let activation = fixture.activate(&mut registry, &enrollment, ConnectorInstanceId::new());
    registry
        .publish(
            activation,
            publication(activation, FIRST_PUBLICATION_SEQUENCE),
            fixture.admission(),
            NOW,
        )
        .unwrap();
    let baseline = registry.snapshots().unwrap();
    let (seed, state) = registry.entry(enrollment.connector_id).unwrap();
    let path = fixture.member(enrollment.connector_id);
    let record = fixture.read(&path);
    fixture.write(&path, &record);
    assert_eq!(
        registry.save(&seed, state),
        Err(ConnectorError::OutcomeUnknown)
    );
    assert!(registry.live.is_empty());
    assert!(registry.snapshots().is_err());
    let stale = registry.last_known();
    assert_eq!(stale.len(), baseline.len());
    assert_eq!(stale[0].cursor().target(), baseline[0].cursor().target());
    assert_eq!(stale[0].fields().freshness, Freshness::Stale);
    assert_eq!(stale[0].fields().reachability, Reachability::Unreachable);
    assert_eq!(
        stale[0].fields().capabilities,
        AgentCapabilities::UNSUPPORTED
    );
    assert_eq!(stale[0].fields().recovery, RecoveryStatus::Unsupported);
    assert_eq!(
        registry
            .last_known_catalogue()
            .primaries
            .get(&activation.terminal),
        Some(&activation.agent)
    );
    assert!(registry.active_scope(activation).is_err());
}

#[test]
fn sequence_exhaustion_never_wraps_and_staleness_uses_the_reserved_final_cursor() {
    let fixture = Fixture::new();
    let mut registry = fixture.open();
    let enrollment = fixture.register(&mut registry);
    let activation = fixture.activate(&mut registry, &enrollment, ConnectorInstanceId::new());
    registry
        .publish(
            activation,
            publication(activation, FIRST_PUBLICATION_SEQUENCE),
            fixture.admission(),
            NOW,
        )
        .unwrap();
    let (seed, mut state) = registry.entry(enrollment.connector_id).unwrap();
    state.last_sequence = MAX_PUBLICATION_SEQUENCE;
    state.last_publication.as_mut().unwrap().sequence = MAX_PUBLICATION_SEQUENCE;
    super::support::snapshot(&mut state, |f| {
        f.cursor = ObservationCursor::new(
            f.cursor.target(),
            f.cursor.stream(),
            MAX_PUBLICATION_SEQUENCE,
        )
        .unwrap()
    });
    registry.save(&seed, state).unwrap();
    let version = fixture.version(&fixture.member(enrollment.connector_id));
    assert_eq!(
        registry.publish(
            activation,
            publication(activation, u64::MAX),
            fixture.admission(),
            NOW
        ),
        Err(ConnectorError::Sequence)
    );
    assert_eq!(
        version,
        fixture.version(&fixture.member(enrollment.connector_id))
    );
    registry.disconnect(activation).unwrap();
    assert_eq!(
        registry.snapshots().unwrap()[0].cursor().sequence(),
        u64::MAX
    );
    registry.disconnect(activation).unwrap();
}

#[test]
fn corrupt_member_reopen_rejects_fresh_snapshot_without_a_publication() {
    let fixture = Fixture::new();
    let mut registry = fixture.open();
    let enrollment = fixture.register(&mut registry);
    let activation = fixture.activate(&mut registry, &enrollment, ConnectorInstanceId::new());
    registry
        .publish(
            activation,
            publication(activation, FIRST_PUBLICATION_SEQUENCE),
            fixture.admission(),
            NOW,
        )
        .unwrap();
    let (seed, mut state) = registry.entry(enrollment.connector_id).unwrap();
    drop(registry);
    state.last_publication = None;
    state.last_sequence = 0;
    fixture.write(
        &fixture.member(seed.connector_id),
        &serde_json::json!({SEED_FIELD: seed, STATE_FIELD: state}),
    );
    let result = crate::Registry::open(
        yolo_paths::directory::PrivateDirectory::open(&fixture.path).unwrap(),
        fixture.identity,
    );
    assert!(matches!(result, Err(ConnectorError::Unavailable)));
}
