use crate::{CONFIG_SCHEMA_VERSION, ConnectionDocument, ReadyDocument};
use yolo_hub_identity::HubId;
mod bootstrap_document;
#[path = "../test_support/constants.rs"]
mod constants;
mod failure;
mod readiness;
mod retry;
use constants::REMOVED_ORIGIN_MAPPING;

const SOCKET: &str = "/tmp/hub.sock";

#[test]
fn startup_document_carries_only_current_public_binding() {
    let hub = HubId::new();
    let document = ConnectionDocument::local(hub, SOCKET.into());
    assert_eq!(document.schema_version, CONFIG_SCHEMA_VERSION);
    assert_eq!(document.entry_hub, document.local_origin);
    let data = serde_json::to_vec(&document).unwrap();
    assert_eq!(
        serde_json::from_slice::<ConnectionDocument>(&data).unwrap(),
        document
    );
}

#[test]
fn readiness_document_keeps_socket_and_identity_together() {
    let document = ReadyDocument {
        schema_version: CONFIG_SCHEMA_VERSION,
        entry_hub: HubId::new(),
        socket_path: SOCKET.into(),
    };
    assert_eq!(
        serde_json::from_slice::<ReadyDocument>(&serde_json::to_vec(&document).unwrap()).unwrap(),
        document
    );
}

#[test]
fn startup_document_rejects_removed_origin_mapping_fields() {
    let document = ConnectionDocument::local(HubId::new(), SOCKET.into());
    let mut wire = serde_json::to_value(document).unwrap();
    wire[REMOVED_ORIGIN_MAPPING] = serde_json::json!([]);
    assert!(serde_json::from_value::<ConnectionDocument>(wire).is_err());
}

#[tokio::test]
async fn expired_budget_cannot_start_a_new_process_or_retry_probe() {
    let deadline = tokio::time::Instant::now();
    assert!(crate::retry::require_budget(deadline).is_err());
    assert!(crate::retry::pause(deadline).await.is_err());
}

#[cfg(target_os = "linux")]
mod host_custody_tests;
