use yolo_hub_remote_startup_test_support::*;
use yolo_remote_provisioning_protocol::*;

#[test]
fn documents_roundtrip_bounded_and_never_contain_process_or_key_file_material() {
    let request = request();
    let ready = RemoteBootstrapReceipt::new(receipt(&request)).unwrap();
    let bytes = serde_json::to_vec(&request).unwrap();
    assert!(bytes.len() < MAX_BOOTSTRAP_DOCUMENT_BYTES);
    assert_eq!(
        serde_json::from_slice::<RemoteBootstrapRequest>(&bytes).unwrap(),
        request
    );
    let bytes = serde_json::to_vec(&ready).unwrap();
    assert!(bytes.len() < MAX_BOOTSTRAP_DOCUMENT_BYTES);
    assert_eq!(
        serde_json::from_slice::<RemoteBootstrapReceipt>(&bytes).unwrap(),
        ready
    );
    let mut value = serde_json::to_value(&ready).unwrap();
    value[UNKNOWN_FIELD] = true.into();
    assert!(serde_json::from_value::<RemoteBootstrapReceipt>(value).is_err());
    let mut value = serde_json::to_value(&request).unwrap();
    value[SOURCE_PATH_FIELD] = true.into();
    assert!(serde_json::from_value::<RemoteBootstrapRequest>(value).is_err());
}

#[test]
fn duplicate_identity_fields_and_invalid_ids_are_not_normalized_by_decoding() {
    let request = request();
    let mut wire = serde_json::to_string(&request).unwrap();
    let duplicate = format!("\"{SCHEMA_FIELD}\":{REMOTE_BOOTSTRAP_SCHEMA_VERSION},");
    wire.insert_str(1, &duplicate);
    assert!(serde_json::from_str::<RemoteBootstrapRequest>(&wire).is_err());
    let mut value = serde_json::to_value(&request).unwrap();
    value[REQUEST_FIELD] = serde_json::to_value(uuid::Uuid::nil()).unwrap();
    assert!(serde_json::from_value::<RemoteBootstrapRequest>(value).is_err());
    let mut value = serde_json::to_value(&request).unwrap();
    value.as_object_mut().unwrap().remove(EXPECTED_PEER_FIELD);
    assert!(serde_json::from_value::<RemoteBootstrapRequest>(value).is_err());
}
