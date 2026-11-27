use super::fixtures;
use crate::*;

#[path = "../test_support/constants.rs"]
mod constants;
use constants::*;

#[test]
fn nested_acknowledgement_objects_reject_unknown_fields_and_missing_binding_fields() {
    let value = serde_json::to_value(fixtures::opened()).unwrap();
    for path in [
        vec![],
        vec![REQUEST_FIELD],
        vec![REQUEST_FIELD, BINDING_FIELD],
        vec![REQUEST_FIELD, BINDING_FIELD, PROFILE_FIELD],
    ] {
        let mut changed = value.clone();
        let mut object = &mut changed;
        for field in path {
            object = object.get_mut(field).unwrap();
        }
        object[UNEXPECTED_FIELD] = serde_json::Value::Bool(true);
        assert!(serde_json::from_value::<BrokerHubOpened>(changed).is_err());
    }
    let mut missing = value;
    missing[REQUEST_FIELD][BINDING_FIELD]
        .as_object_mut()
        .unwrap()
        .remove(GENERATION_FIELD);
    assert!(serde_json::from_value::<BrokerHubOpened>(missing).is_err());
}

#[test]
fn decode_cannot_turn_an_ssh_path_or_foreign_config_into_the_expected_tunnel() {
    let request = fixtures::request();
    let opened = BrokerHubOpened {
        request: request.clone(),
        channel_id: fixtures::CHANNEL,
    };
    let mut value = serde_json::to_value(&opened).unwrap();
    value[REQUEST_FIELD][ENDPOINT_FIELD] = serde_json::Value::String("/srv/hub.sock".into());
    assert!(serde_json::from_value::<BrokerHubOpened>(value).is_err());
    let mut value = serde_json::to_value(opened).unwrap();
    value[REQUEST_FIELD][BINDING_FIELD][PROFILE_FIELD][REVISION_FIELD] =
        serde_json::Value::String(FOREIGN_REVISION.into());
    let foreign: BrokerHubOpened = serde_json::from_value(value).unwrap();
    foreign.request.validate().unwrap();
    assert_eq!(
        foreign.validate(&request),
        Err(BrokerHubError::ChannelMismatch)
    );
}
