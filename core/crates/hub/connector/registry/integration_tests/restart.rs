use crate::{constants::*, fixture::*};
use yolo_hub_agent_protocol::{AgentCapabilities, Freshness, RecoveryStatus};
use yolo_hub_connector_protocol::{ConnectorError, FIRST_PUBLICATION_SEQUENCE};
use yolo_hub_connector_registry::Registry;
use yolo_hub_identity::ConnectorInstanceId;
use yolo_paths::directory::PrivateDirectory;

#[test]
fn reopening_stales_retained_snapshot_and_never_restores_a_live_connection_generation() {
    let fixture = Fixture::new();
    let mut registry = fixture.open();
    let enrollment = fixture.register(&mut registry);
    let instance = ConnectorInstanceId::new();
    let active = fixture.activate(&mut registry, &enrollment, instance);
    registry
        .publish(
            active,
            publication(active, FIRST_PUBLICATION_SEQUENCE),
            fixture.admission(),
            NOW,
        )
        .unwrap();
    let first = registry.snapshots().unwrap()[0].clone();
    drop(registry);
    let mut reopened = fixture.open();
    let retained = reopened.snapshots().unwrap()[0].clone();
    assert_eq!(retained.cursor().target(), first.cursor().target());
    assert_eq!(retained.fields().freshness, Freshness::Stale);
    assert!(retained.cursor().sequence() > first.cursor().sequence());
    assert_eq!(
        reopened.active_scope(active),
        Err(ConnectorError::StaleGeneration)
    );
    assert_eq!(
        reopened.publish(
            active,
            publication(active, FIRST_PUBLICATION_SEQUENCE + STEP),
            fixture.admission(),
            LATER
        ),
        Err(ConnectorError::StaleGeneration)
    );
    let replacement = fixture.activate(&mut reopened, &enrollment, instance);
    assert_eq!(replacement.agent, active.agent);
    assert_ne!(replacement.generation, active.generation);
    reopened
        .publish(
            replacement,
            publication(replacement, FIRST_PUBLICATION_SEQUENCE),
            fixture.admission(),
            LATER,
        )
        .unwrap();
}

#[test]
fn corrupted_store_can_only_yield_stale_last_known_metadata_and_no_mutation_authority() {
    let fixture = Fixture::new();
    let mut registry = fixture.open();
    let enrollment = fixture.register(&mut registry);
    let active = fixture.activate(&mut registry, &enrollment, ConnectorInstanceId::new());
    registry
        .publish(
            active,
            publication(active, FIRST_PUBLICATION_SEQUENCE),
            fixture.admission(),
            NOW,
        )
        .unwrap();
    std::fs::write(fixture.member(enrollment.connector_id), CORRUPT_BYTES).unwrap();
    assert!(registry.catalogue().is_err());
    assert!(registry.active_scope(active).is_err());
    let fallback = registry.last_known_catalogue();
    assert_eq!(fallback.agents[0].fields().freshness, Freshness::Stale);
    assert_eq!(
        fallback.agents[0].fields().recovery,
        RecoveryStatus::Unsupported
    );
    assert_eq!(
        fallback.agents[0].fields().capabilities,
        AgentCapabilities::UNSUPPORTED
    );
    assert_eq!(
        fallback.primaries.get(&active.terminal),
        Some(&active.agent)
    );
    drop(registry);
    assert!(matches!(
        Registry::open(
            PrivateDirectory::open(&fixture.path).unwrap(),
            fixture.identity
        ),
        Err(ConnectorError::Unavailable)
    ));
}

#[test]
fn replacement_namespace_does_not_regrant_registration_or_activate_prior_identity() {
    let fixture = Fixture::new();
    let mut registry = fixture.open();
    let enrollment = fixture.register(&mut registry);
    let retired = fixture
        .root
        .path()
        .canonicalize()
        .unwrap()
        .join(RETIRED_DIRECTORY);
    std::fs::rename(&fixture.path, &retired).unwrap();
    PrivateDirectory::ensure(&fixture.path).unwrap();
    assert!(registry.list().is_err());
    assert!(
        registry
            .activate(
                enrollment.connector_id,
                enrollment.credential,
                ConnectorInstanceId::new(),
                fixture.admission()
            )
            .is_err()
    );
    assert!(!fixture.path.join(INDEX_FILE).exists());
}
