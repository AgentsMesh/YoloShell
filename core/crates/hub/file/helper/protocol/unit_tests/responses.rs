use crate::fixtures::*;
use yolo_hub_file_helper_protocol::*;
use yolo_hub_terminal_protocol::PublicError;

#[test]
fn replies_require_exact_schema_origin_correlation_and_payload_kind() {
    for fields in [inspect(), read(), build()] {
        let request = Request::new(fields).unwrap();
        let response = Response::new(&request, payload(&request));
        response.validate_for(&request).unwrap();
        roundtrip(&response);
        let mut wrong = response.clone();
        wrong.schema_version = u16::MAX;
        assert_eq!(
            wrong.validate_for(&request),
            Err(PublicError::UnsupportedSchema)
        );
        let mut wrong = response.clone();
        wrong.request_id = uuid::Uuid::new_v4();
        assert_eq!(
            wrong.validate_for(&request),
            Err(PublicError::InvalidRequest)
        );
        let mut wrong = response.clone();
        wrong.origin = yolo_hub_identity::HubId::new();
        assert_eq!(
            wrong.validate_for(&request),
            Err(PublicError::OriginMismatch)
        );
        Response::new(
            &request,
            Payload::Error {
                code: PublicError::Conflict,
            },
        )
        .validate_for(&request)
        .unwrap();
        let other = Request::new(match &request.fields().command {
            Command::Build {} => inspect(),
            Command::Inspect { .. } => read(),
            Command::Read { .. } => inspect(),
        })
        .unwrap();
        let wrong = Response::new(&request, payload(&other));
        assert_eq!(
            wrong.validate_for(&request),
            Err(PublicError::InvalidPayload)
        );
    }
}

#[test]
fn an_inspection_reply_cannot_omit_or_substitute_an_approved_root() {
    let request = Request::new(inspect()).unwrap();
    let incomplete = Response::new(&request, Payload::Inspected { bindings: vec![] });
    assert_eq!(
        incomplete.validate_for(&request),
        Err(PublicError::TargetMismatch)
    );
    let substituted = Response::new(
        &request,
        Payload::Inspected {
            bindings: vec![binding(root())],
        },
    );
    assert_eq!(
        substituted.validate_for(&request),
        Err(PublicError::TargetMismatch)
    );
    let mut object = serde_json::to_value(Response::new(&request, payload(&request))).unwrap();
    object[EXTRA_FIELD] = true.into();
    assert!(serde_json::from_value::<Response>(object).is_err());
}
