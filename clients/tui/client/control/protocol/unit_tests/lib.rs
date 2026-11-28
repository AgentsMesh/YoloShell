use super::*;
mod constants;

#[test]
fn messages_round_trip_and_authorize_only_current_nonempty_token() {
    let token = Uuid::new_v4();
    let request = ControlRequest::new(token);
    assert!(request.authorizes(token));
    assert!(!request.authorizes(Uuid::new_v4()));
    assert!(!ControlRequest::new(Uuid::nil()).authorizes(Uuid::nil()));
    assert_eq!(
        serde_json::from_slice::<ControlRequest>(&serde_json::to_vec(&request).unwrap()).unwrap(),
        request
    );
    assert!(ControlResponse::accepted().confirms_detached());
    assert!(!ControlResponse::rejected().confirms_detached());
}

#[test]
fn descriptors_and_requests_reject_invalid_versions_and_unknown_fields() {
    let mut descriptor = ControlDescriptor::new(Uuid::new_v4(), Uuid::new_v4());
    assert!(descriptor.is_valid());
    descriptor.version = u16::MAX;
    assert!(!descriptor.is_valid());
    descriptor.version = CONTROL_PROTOCOL_VERSION;
    descriptor.endpoint = Uuid::nil();
    assert!(!descriptor.is_valid());
    descriptor.endpoint = Uuid::new_v4();
    descriptor.token = Uuid::nil();
    assert!(!descriptor.is_valid());
    let mut request = ControlRequest::new(Uuid::new_v4());
    request.version = u16::MAX;
    assert!(!request.authorizes(request.token));
    let mut encoded = serde_json::to_value(request).unwrap();
    encoded["unexpected"] = true.into();
    assert!(serde_json::from_value::<ControlRequest>(encoded).is_err());
    let mut encoded = serde_json::to_value(descriptor).unwrap();
    encoded[constants::UNRECOGNIZED_SOCKET_PATH_FIELD] = constants::UNRECOGNIZED_SOCKET_PATH.into();
    assert!(serde_json::from_value::<ControlDescriptor>(encoded).is_err());
}
