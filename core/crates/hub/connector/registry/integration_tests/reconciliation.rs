use crate::{constants::*, fixture::*};
use std::collections::BTreeMap;
use yolo_hub_agent_protocol::{Freshness, Reachability};
use yolo_hub_connector_protocol::{ConnectorError, EnrollmentStatus, FIRST_PUBLICATION_SEQUENCE};
use yolo_hub_connector_registry::RuntimeAdmission;
use yolo_hub_identity::{ConnectorInstanceId, RuntimeEpoch};

#[test]
fn native_ownership_retirement_or_replaced_runtime_permanently_supersedes_enrollment() {
    let fixture = Fixture::new();
    let mut replaced = fixture.scope.terminal;
    replaced.runtime_epoch = RuntimeEpoch::new();
    for fact in [
        RuntimeAdmission::NativeOwned,
        RuntimeAdmission::Retired,
        RuntimeAdmission::Available(replaced),
    ] {
        let isolated = Fixture::new();
        let mut registry = isolated.open();
        let enrollment = isolated.register(&mut registry);
        let active = isolated.activate(&mut registry, &enrollment, ConnectorInstanceId::new());
        registry
            .publish(
                active,
                publication(active, FIRST_PUBLICATION_SEQUENCE),
                isolated.admission(),
                NOW,
            )
            .unwrap();
        registry
            .reconcile(&BTreeMap::from([(active.terminal, fact)]))
            .unwrap();
        let listing = registry.list().unwrap();
        assert_eq!(listing[0].status, EnrollmentStatus::Superseded);
        assert_eq!(listing[0].revision, enrollment.revision + STEP);
        assert!(registry.catalogue().unwrap().primaries.is_empty());
        assert!(registry.last_known_catalogue().primaries.is_empty());
        let stale = registry.snapshots().unwrap();
        assert_eq!(stale[0].fields().freshness, Freshness::Stale);
        assert_eq!(stale[0].fields().reachability, Reachability::Unreachable);
        registry
            .reconcile(&BTreeMap::from([(active.terminal, isolated.admission())]))
            .unwrap();
        assert_eq!(
            registry.activate(
                enrollment.connector_id,
                enrollment.credential,
                ConnectorInstanceId::new(),
                isolated.admission()
            ),
            Err(ConnectorError::Unauthorized)
        );
    }
}

#[test]
fn unavailable_or_missing_runtime_fact_deactivates_until_explicit_reactivation() {
    for missing in [false, true] {
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
        let facts = if missing {
            BTreeMap::new()
        } else {
            BTreeMap::from([(active.terminal, RuntimeAdmission::Unavailable)])
        };
        registry.reconcile(&facts).unwrap();
        assert_eq!(
            registry.list().unwrap()[0].status,
            EnrollmentStatus::Enabled
        );
        assert_eq!(registry.list().unwrap()[0].revision, enrollment.revision);
        assert!(registry.active_scope(active).is_err());
        registry
            .reconcile(&BTreeMap::from([(active.terminal, fixture.admission())]))
            .unwrap();
        assert!(registry.active_scope(active).is_err());
        let next = fixture.activate(&mut registry, &enrollment, instance);
        assert_eq!(next.agent, active.agent);
        assert_ne!(next.generation, active.generation);
        registry
            .publish(
                next,
                publication(next, FIRST_PUBLICATION_SEQUENCE),
                fixture.admission(),
                LATER,
            )
            .unwrap();
    }
}

#[test]
fn native_conflict_during_publication_never_commits_connector_observation() {
    let fixture = Fixture::new();
    let mut registry = fixture.open();
    let enrollment = fixture.register(&mut registry);
    let active = fixture.activate(&mut registry, &enrollment, ConnectorInstanceId::new());
    assert_eq!(
        registry.publish(
            active,
            publication(active, FIRST_PUBLICATION_SEQUENCE),
            RuntimeAdmission::NativeOwned,
            NOW
        ),
        Err(ConnectorError::Conflict)
    );
    assert!(registry.snapshots().unwrap().is_empty());
    assert_eq!(
        registry.list().unwrap()[0].status,
        EnrollmentStatus::Superseded
    );
}

#[test]
fn disconnect_all_stales_every_observation_but_keeps_display_only_primary_associations() {
    let fixture = Fixture::new();
    let mut registry = fixture.open();
    let first = fixture.register(&mut registry);
    let first_active = fixture.activate(&mut registry, &first, ConnectorInstanceId::new());
    let mut second_scope = fixture.scope.clone();
    second_scope.terminal.address.terminal_id = yolo_hub_identity::TerminalId::new();
    let second_admission = RuntimeAdmission::Available(second_scope.terminal);
    let second = registry
        .register(uuid::Uuid::new_v4(), second_scope, second_admission)
        .unwrap();
    let second_active = registry
        .activate(
            second.connector_id,
            second.credential,
            ConnectorInstanceId::new(),
            second_admission,
        )
        .unwrap();
    for (active, admission) in [
        (first_active, fixture.admission()),
        (second_active, second_admission),
    ] {
        registry
            .publish(
                active,
                publication(active, FIRST_PUBLICATION_SEQUENCE),
                admission,
                NOW,
            )
            .unwrap();
    }
    registry.disconnect_all().unwrap();
    registry.disconnect_all().unwrap();
    for active in [first_active, second_active] {
        assert!(registry.active_scope(active).is_err());
    }
    let catalogue = registry.catalogue().unwrap();
    assert_eq!(catalogue.primaries.len(), catalogue.agents.len());
    assert!(
        catalogue
            .agents
            .iter()
            .all(|agent| agent.fields().freshness == Freshness::Stale)
    );
    assert!(catalogue.enabled_targets.is_empty());
}
