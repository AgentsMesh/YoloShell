use crate::{constants::*, fixture::Fixture};
use yolo_hub_agent_protocol::ObservationCursor;
use yolo_hub_connector_protocol::*;
use yolo_hub_identity::{AgentId, ConnectorGeneration, ConnectorId, ExecutionId, HubId};

#[test]
fn acknowledgements_fence_every_publication_identity_without_conflating_journal_sequence() {
    let fixture = Fixture::new();
    let request = Request::new(
        fixture.hub,
        Command::Publish {
            publication: fixture.publication.clone(),
        },
    )
    .unwrap();
    assert_ne!(fixture.ack.sequence, fixture.ack.cursor.sequence());
    for duplicate in [false, true] {
        Response::new(
            &request,
            Reply::Published {
                ack: PublicationAck {
                    duplicate,
                    ..fixture.ack
                },
            },
        )
        .validate_for(&request)
        .unwrap();
    }
    let mutations: &[fn(&mut PublicationAck)] = &[
        |a| a.connector_id = ConnectorId::new(),
        |a| a.generation = ConnectorGeneration::new(),
        |a| a.sequence += FIRST_PUBLICATION_SEQUENCE,
        |a| {
            let mut target = a.cursor.target();
            target.address.agent_id = AgentId::new();
            a.cursor =
                ObservationCursor::new(target, a.cursor.stream(), a.cursor.sequence()).unwrap();
        },
        |a| {
            let mut target = a.cursor.target();
            target.execution_id = ExecutionId::new();
            a.cursor =
                ObservationCursor::new(target, a.cursor.stream(), a.cursor.sequence()).unwrap();
        },
        |a| {
            let mut target = a.cursor.target();
            target.address.hub_id = HubId::new();
            a.cursor =
                ObservationCursor::new(target, a.cursor.stream(), a.cursor.sequence()).unwrap();
        },
    ];
    for mutate in mutations {
        let mut changed = fixture.ack;
        mutate(&mut changed);
        assert_eq!(
            Response::new(&request, Reply::Published { ack: changed }).validate_for(&request),
            Err(ConnectorError::InvalidRequest)
        );
    }
}

#[test]
fn lists_are_bounded_and_reject_duplicate_connector_ids_and_foreign_scopes() {
    let fixture = Fixture::new();
    let request = Request::new(fixture.hub, Command::List {}).unwrap();
    let summary = fixture.summary(EnrollmentStatus::Enabled);
    for count in [0, MAX_CONNECTORS] {
        let entries = (0..count)
            .map(|_| EnrollmentSummary {
                connector_id: ConnectorId::new(),
                ..summary.clone()
            })
            .collect();
        Response::new(
            &request,
            Reply::Enrollments {
                enrollments: entries,
            },
        )
        .validate_for(&request)
        .unwrap();
    }
    let oversized = Response::new(
        &request,
        Reply::Enrollments {
            enrollments: vec![summary.clone(); MAX_CONNECTORS + 1],
        },
    );
    assert_eq!(
        oversized.validate_for(&request),
        Err(ConnectorError::Capacity)
    );
    assert!(
        serde_json::from_value::<Reply>(serde_json::to_value(oversized.reply).unwrap()).is_err()
    );
    let duplicates = Response::new(
        &request,
        Reply::Enrollments {
            enrollments: vec![summary.clone(), summary.clone()],
        },
    );
    assert_eq!(
        duplicates.validate_for(&request),
        Err(ConnectorError::InvalidRequest)
    );
    assert!(
        serde_json::from_value::<Reply>(serde_json::to_value(duplicates.reply).unwrap()).is_err()
    );
    let mut foreign = summary;
    foreign.agent.hub_id = HubId::new();
    foreign.scope.terminal.address.hub_id = foreign.agent.hub_id;
    assert_eq!(
        Response::new(
            &request,
            Reply::Enrollments {
                enrollments: vec![foreign]
            }
        )
        .validate_for(&request),
        Err(ConnectorError::InvalidRequest)
    );
}
