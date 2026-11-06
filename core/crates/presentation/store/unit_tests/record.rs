use super::{fixture::*, *};
use crate::{constants::*, record::Record};
use yolo_hub_identity::HubId;
use yolo_presentation_protocol::ProtocolError;

#[test]
fn store_record_validates_schema_origin_identity_and_capacity() {
    let hub = HubId::new();
    let original = Record::empty(hub);
    original.validate(hub).unwrap();
    assert!(original.validate(HubId::new()).is_err());
    let mut record = original.clone();
    record.schema_version = SCHEMA_VERSION + 1;
    assert!(record.validate(hub).is_err());
    record.schema_version = SCHEMA_VERSION - 1;
    assert!(record.validate(hub).is_err());
    let document = update(hub).apply(None).unwrap();
    record = original.clone();
    record.documents = vec![document.clone(), document.clone()];
    assert!(record.validate(hub).is_err());
    record.documents = vec![document.clone(); MAX_DOCUMENTS + 1];
    assert!(record.validate(hub).is_err());
    record.documents = vec![document];
    record.documents[0].address.hub_id = HubId::new();
    assert!(record.validate(hub).is_err());
    record.documents[0].revision = 0;
    assert!(record.validate(hub).is_err());
}

#[test]
fn errors_preserve_protocol_vs_storage_causes() {
    for error in [
        StoreError::from(ProtocolError::RevisionConflict),
        StoreError::from(anyhow::anyhow!(LOST)),
    ] {
        assert!(!error.to_string().is_empty());
        assert!(std::error::Error::source(&error).is_some());
    }
}
