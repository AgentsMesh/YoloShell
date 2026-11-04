use super::{constants::*, fixture::*};
use crate::{Registry, state::State};
use yolo_hub_connector_protocol::{ConnectorError, FIRST_PUBLICATION_SEQUENCE};
use yolo_hub_identity::ConnectorInstanceId;
use yolo_paths::directory::PrivateDirectory;

pub fn rejects_persisted_change(mutate: impl FnOnce(&mut State)) {
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
    let (seed, mut state) = registry.entry(enrolled.connector_id).unwrap();
    drop(registry);
    mutate(&mut state);
    let path = fixture.member(enrolled.connector_id);
    fixture.write(
        &path,
        &serde_json::json!({SEED_FIELD: seed, STATE_FIELD: state}),
    );
    let before = fixture.version(&path);
    assert!(matches!(
        Registry::open(
            PrivateDirectory::open(&fixture.path).unwrap(),
            fixture.identity
        ),
        Err(ConnectorError::Unavailable)
    ));
    assert_eq!(fixture.version(&path), before);
}

pub fn mark_exited(state: &mut State) {
    let last = state.last_publication.take().unwrap();
    state.last_publication = Some(exited(last));
    super::support::snapshot(state, |fields| {
        fields.lifecycle = yolo_hub_agent_protocol::Lifecycle::Exited {
            exit_code: Some(EXIT_CODE),
        };
        fields.activity = yolo_hub_agent_protocol::Activity::Idle;
    });
    state.source_exited = true;
}

const EXIT_CODE: i32 = 0;
