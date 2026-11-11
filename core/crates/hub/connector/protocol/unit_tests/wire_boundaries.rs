use crate::{constants::*, fixture::Fixture};
use yolo_hub_connector_protocol::*;

#[test]
fn enrollment_summaries_never_accept_secret_fields_and_invalid_status_is_not_revocation() {
    let fixture = Fixture::new();
    let summary = fixture.summary(EnrollmentStatus::Enabled);
    let mut wire = serde_json::to_value(&summary).unwrap();
    assert!(wire.get(field::CREDENTIAL).is_none());
    wire[field::CREDENTIAL] = serde_json::to_value(fixture.enrollment.credential).unwrap();
    assert!(serde_json::from_value::<EnrollmentSummary>(wire).is_err());
    let invalid = Reply::Revoked {
        enrollment: summary,
    };
    assert_eq!(invalid.validate(), Err(ConnectorError::InvalidRequest));
    assert!(serde_json::from_value::<Reply>(serde_json::to_value(invalid).unwrap()).is_err());
}

#[test]
fn public_dtos_reject_nested_unknown_fields_and_malformed_newtype_identities() {
    let fixture = Fixture::new();
    let request = Request::new(
        fixture.hub,
        Command::Publish {
            publication: fixture.publication.clone(),
        },
    )
    .unwrap();
    let original = serde_json::to_value(request).unwrap();
    for path in [
        vec![field::COMMAND, field::PUBLICATION],
        vec![field::COMMAND, field::PUBLICATION, field::AGENT],
        vec![field::COMMAND, field::PUBLICATION, field::TERMINAL],
        vec![field::COMMAND, field::PUBLICATION, field::STATE],
    ] {
        let mut wire = original.clone();
        let mut node = &mut wire;
        for field in path {
            node = node.get_mut(field).unwrap();
        }
        node.as_object_mut()
            .unwrap()
            .insert(UNKNOWN_FIELD.into(), true.into());
        assert!(serde_json::from_value::<Request>(wire).is_err());
    }
    for key in [field::CONNECTOR_ID, field::GENERATION] {
        let mut wire = original.clone();
        wire[field::COMMAND][field::PUBLICATION][key] = uuid::Uuid::nil().to_string().into();
        assert!(serde_json::from_value::<Request>(wire).is_err());
    }
    let mut scope = serde_json::to_value(&fixture.enrollment.scope).unwrap();
    scope[UNKNOWN_FIELD] = true.into();
    assert!(serde_json::from_value::<ConnectorScope>(scope).is_err());
}

#[test]
fn response_validation_cannot_approve_an_invalid_request_or_nil_response_id() {
    let fixture = Fixture::new();
    let mut request = Request::new(fixture.hub, Command::Hello {}).unwrap();
    let mut response = Response::new(&request, Reply::Hello { hub: fixture.hub });
    response.request_id = uuid::Uuid::nil();
    assert_eq!(
        response.validate_for(&request),
        Err(ConnectorError::InvalidRequest)
    );
    assert!(serde_json::to_value(&response).is_err());
    request.version = 0;
    response.request_id = request.request_id;
    assert_eq!(
        response.validate_for(&request),
        Err(ConnectorError::UnsupportedSchema)
    );
}
