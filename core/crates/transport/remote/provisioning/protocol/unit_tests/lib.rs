#[path = "../test_support/constants.rs"]
mod constants;
#[path = "../test_support/fixture.rs"]
mod fixture;
mod bootstrap_correlation;
mod bootstrap_recovery;
mod bootstrap_validation;
mod bootstrap_wire;
use yolo_remote_provisioning_protocol::*;

#[test]
fn bounded_locator_and_full_profile_identity_roundtrip_without_payload_or_credentials() {
    let call = fixture::call();
    call.validate().unwrap();
    let bytes = serde_json::to_vec(&call).unwrap();
    assert_eq!(
        serde_json::from_slice::<BrokerProvisionCall>(&bytes).unwrap(),
        call
    );
    let mut changed = call.clone();
    changed.binding.profile.host_id.push('x');
    assert!(changed.validate().is_err());
    let mut fields = call.request.fields().clone();
    fields.request_id = uuid::Uuid::new_v4();
    assert!(ProvisionRequest::new(fields).is_err());
    let mut fields = call.request.fields().clone();
    fields.state = fields
        .install_root
        .join(yolo_distribution_bundle::RELEASES_DIRECTORY);
    assert!(ProvisionRequest::new(fields).is_err());
    let mut fields = call.request.fields().clone();
    fields.host_home = fields.state.clone();
    assert!(ProvisionRequest::new(fields).is_err());
    let mut wire = serde_json::to_value(&call.request).unwrap();
    wire[constants::PRIVATE_FIELD] = true.into();
    assert!(serde_json::from_value::<ProvisionRequest>(wire).is_err());
    let mut wire = serde_json::to_value(&call.request).unwrap();
    wire[constants::SOURCE_FIELD][constants::ROOT_FIELD] = "relative".into();
    assert!(serde_json::from_value::<ProvisionRequest>(wire).is_err());
}

#[test]
fn status_never_confirms_another_request_or_an_invalid_schema() {
    let call = fixture::call();
    for outcome in [
        ProvisionOutcome::Absent {},
        ProvisionOutcome::Unknown {
            phase: ProvisionPhase::Publishing,
        },
        ProvisionOutcome::Superseded {
            phase: ProvisionPhase::Enrolling,
        },
    ] {
        let status = ProvisionStatus::new(call.request.clone(), outcome).unwrap();
        status.validate_for(&call.request).unwrap();
        assert!(status.validate_for(&fixture::call().request).is_err());
        let mut wire = serde_json::to_value(status).unwrap();
        wire[constants::SCHEMA_FIELD] = u16::MAX.into();
        assert!(serde_json::from_value::<ProvisionStatus>(wire).is_err());
    }
}
