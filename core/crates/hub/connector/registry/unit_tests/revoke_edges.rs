use super::{constants::*, fixture::*};
use crate::RuntimeAdmission;
use yolo_hub_connector_protocol::{ConnectorError, EnrollmentStatus, FIRST_PUBLICATION_SEQUENCE};
use yolo_hub_identity::ConnectorInstanceId;

#[test]
fn nil_revocation_or_wrong_revision_cannot_stale_live_observation_or_retire_generation() {
    let fixture = Fixture::new();
    let mut registry = fixture.open();
    let enrolled = fixture.register(&mut registry);
    let active = fixture.activate(&mut registry, &enrolled, ConnectorInstanceId::new());
    registry
        .publish(
            active,
            publication(active, FIRST_PUBLICATION_SEQUENCE),
            fixture.admission(),
            NOW,
        )
        .unwrap();
    let before = registry.snapshots().unwrap();
    let version = fixture.version(&fixture.member(enrolled.connector_id));
    assert_eq!(
        registry.revoke(uuid::Uuid::nil(), enrolled.connector_id, enrolled.revision),
        Err(ConnectorError::InvalidRequest)
    );
    assert_eq!(
        registry.revoke(
            uuid::Uuid::new_v4(),
            enrolled.connector_id,
            enrolled.revision + STEP
        ),
        Err(ConnectorError::Conflict)
    );
    assert_eq!(registry.active_scope(active).unwrap(), enrolled.scope);
    assert_eq!(registry.snapshots().unwrap(), before);
    assert_eq!(
        fixture.version(&fixture.member(enrolled.connector_id)),
        version
    );
}

#[test]
fn replaying_old_registration_request_cannot_restore_revoked_or_superseded_enrollment() {
    for native_takeover in [false, true] {
        let fixture = Fixture::new();
        let mut registry = fixture.open();
        let request = uuid::Uuid::new_v4();
        let enrolled = registry
            .register(request, fixture.scope.clone(), fixture.admission())
            .unwrap();
        if native_takeover {
            registry
                .reconcile(
                    &[(enrolled.scope.terminal, RuntimeAdmission::NativeOwned)]
                        .into_iter()
                        .collect(),
                )
                .unwrap();
        } else {
            registry
                .revoke(
                    uuid::Uuid::new_v4(),
                    enrolled.connector_id,
                    enrolled.revision,
                )
                .unwrap();
        }
        let before = fixture.version(&fixture.member(enrolled.connector_id));
        assert_eq!(
            registry.register(request, fixture.scope.clone(), fixture.admission()),
            Err(ConnectorError::Conflict)
        );
        assert_eq!(
            fixture.version(&fixture.member(enrolled.connector_id)),
            before
        );
        let successor = fixture.register(&mut registry);
        assert_ne!(successor.connector_id, enrolled.connector_id);
        assert_ne!(successor.agent, enrolled.agent);
        assert_ne!(successor.credential, enrolled.credential);
    }
}

#[test]
fn exhausted_revision_never_wraps_or_persists_partial_revocation() {
    let fixture = Fixture::new();
    let mut registry = fixture.open();
    let enrolled = fixture.register(&mut registry);
    let (seed, mut state) = registry.entry(enrolled.connector_id).unwrap();
    state.revision = u64::MAX;
    registry.save(&seed, state).unwrap();
    let before = fixture.version(&fixture.member(enrolled.connector_id));
    assert_eq!(
        registry.revoke(uuid::Uuid::new_v4(), enrolled.connector_id, u64::MAX),
        Err(ConnectorError::Capacity)
    );
    let retained = registry.list().unwrap();
    assert_eq!(retained[0].status, EnrollmentStatus::Enabled);
    assert_eq!(retained[0].revision, u64::MAX);
    assert_eq!(
        fixture.version(&fixture.member(enrolled.connector_id)),
        before
    );
}
