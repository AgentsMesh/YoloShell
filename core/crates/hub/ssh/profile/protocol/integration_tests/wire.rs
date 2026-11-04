use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use yolo_hub_ssh_profile_protocol::*;
use yolo_hub_ssh_profile_protocol_test_support::{constants::*, *};

#[test]
fn public_registry_dtos_roundtrip_and_reject_partial_unknown_and_duplicate_objects() {
    strict(profile(0), &[LAUNCH_KEY]);
    strict(configuration(), &[]);
    strict(state(), &[]);
    strict(change(&state(), configuration()), &[]);
}

#[test]
fn malformed_nested_values_are_rejected_during_deserialization() {
    for (key, replacement) in [
        (BROKER_KEY, json!("relative")),
        (CONFIG_REVISION_KEY, json!(INVALID_REVISION)),
    ] {
        let mut value = serde_json::to_value(profile(0)).unwrap();
        value[key] = replacement;
        assert!(serde_json::from_value::<SshBrokerProfile>(value).is_err());
    }
    let mut value = serde_json::to_value(configuration()).unwrap();
    value[SCHEMA_KEY] = json!(SSH_PROFILE_SCHEMA_VERSION + 1);
    assert!(serde_json::from_value::<SshProfileConfiguration>(value).is_err());
    let mut value = serde_json::to_value(state()).unwrap();
    value[REVISION_KEY] = json!(0);
    assert!(serde_json::from_value::<SshProfileState>(value).is_err());
    let mut value = serde_json::to_value(change(&state(), configuration())).unwrap();
    value[EXPECTED_REVISION_KEY] = json!(0);
    assert!(serde_json::from_value::<SshProfileChange>(value).is_err());
}

fn strict<T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug>(
    value: T,
    optional: &[&str],
) {
    let json = serde_json::to_value(&value).unwrap();
    let encoded = serde_json::to_string(&json).unwrap();
    assert_eq!(serde_json::from_str::<T>(&encoded).unwrap(), value);
    for (key, member) in json.as_object().unwrap() {
        let mut missing = json.clone();
        missing.as_object_mut().unwrap().remove(key);
        if optional.contains(&key.as_str()) {
            assert_eq!(serde_json::from_value::<T>(missing).unwrap(), value);
        } else {
            assert!(serde_json::from_value::<T>(missing).is_err());
        }
        let field = format!("{}:{}", serde_json::to_string(key).unwrap(), member);
        let duplicate = encoded.replacen(&field, &format!("{field},{field}"), 1);
        assert!(serde_json::from_str::<T>(&duplicate).is_err());
    }
    let mut unknown = json;
    unknown[EXTRA_KEY] = Value::Bool(true);
    assert!(serde_json::from_value::<T>(unknown).is_err());
}
