use crate::{constants::*, fixture::Fixture};
use uuid::Uuid;
use yolo_hub_connector_protocol::*;
use yolo_hub_identity::HubId;

#[test]
fn all_commands_round_trip_with_canonical_identity_and_mutation_semantics() {
    let fixture = Fixture::new();
    for (request, _) in fixture.samples() {
        request.validate().unwrap();
        assert!(!request.request_id.is_nil());
        assert_eq!(
            request.command.mutates(),
            !matches!(request.command, Command::Hello {} | Command::List {})
        );
        let bytes = serde_json::to_vec(&request).unwrap();
        assert_eq!(serde_json::from_slice::<Request>(&bytes).unwrap(), request);
        let mut unknown = serde_json::to_value(&request).unwrap();
        unknown[UNKNOWN_FIELD] = true.into();
        assert!(serde_json::from_value::<Request>(unknown).is_err());
        let mut unknown = serde_json::to_value(&request).unwrap();
        unknown[field::COMMAND][UNKNOWN_FIELD] = true.into();
        assert!(serde_json::from_value::<Request>(unknown).is_err());
    }
}

#[test]
fn version_and_request_id_validation_reject_downgrades_nil_and_noncanonical_uuid() {
    let fixture = Fixture::new();
    let (request, _) = fixture.samples().remove(0);
    for version in INVALID_PROTOCOL_VERSIONS {
        let changed = Request {
            version,
            ..request.clone()
        };
        assert_eq!(changed.validate(), Err(ConnectorError::UnsupportedSchema));
        assert!(serde_json::from_value::<Request>(serde_json::to_value(changed).unwrap()).is_err());
    }
    for identity in [
        Uuid::nil().to_string(),
        UUID_TEXT.to_uppercase(),
        UUID_TEXT.replace('-', ""),
        UNKNOWN_TEXT.into(),
    ] {
        let mut changed = serde_json::to_value(&request).unwrap();
        changed[field::REQUEST_ID] = identity.into();
        assert!(serde_json::from_value::<Request>(changed).is_err());
    }
    let nil = Request {
        request_id: Uuid::nil(),
        ..request
    };
    assert_eq!(nil.validate(), Err(ConnectorError::InvalidRequest));
    assert!(serde_json::to_vec(&nil).is_err());
}

#[test]
fn register_and_publish_cannot_use_a_foreign_hub_or_malformed_state() {
    let fixture = Fixture::new();
    for command in [
        Command::Register {
            scope: fixture.enrollment.scope.clone(),
        },
        Command::Publish {
            publication: fixture.publication.clone(),
        },
    ] {
        assert_eq!(
            Request::new(HubId::new(), command),
            Err(ConnectorError::InvalidRequest)
        );
    }
    let invalid = Command::Revoke {
        connector_id: fixture.enrollment.connector_id,
        expected_revision: 0,
    };
    assert_eq!(
        Request::new(fixture.hub, invalid.clone()),
        Err(ConnectorError::InvalidRequest)
    );
    assert!(serde_json::from_value::<Command>(serde_json::to_value(invalid).unwrap()).is_err());
    let mut invalid = fixture.publication;
    invalid.sequence = u64::MAX;
    assert_eq!(
        Request::new(
            fixture.hub,
            Command::Publish {
                publication: invalid
            }
        ),
        Err(ConnectorError::Sequence)
    );
}

#[test]
fn canonical_caller_supplied_request_id_survives_serialization_for_idempotent_retry() {
    let fixture = Fixture::new();
    let id = Uuid::parse_str(UUID_TEXT).unwrap();
    let request = Request {
        version: CONNECTOR_PROTOCOL_VERSION,
        request_id: id,
        hub: fixture.hub,
        command: Command::Register {
            scope: fixture.enrollment.scope,
        },
    };
    request.validate().unwrap();
    let decoded =
        serde_json::from_slice::<Request>(&serde_json::to_vec(&request).unwrap()).unwrap();
    assert_eq!(decoded.request_id, id);
    assert_eq!(decoded, request);
}
