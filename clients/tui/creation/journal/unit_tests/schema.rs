use super::{records::document, support::*};
use crate::{JOURNAL_SCHEMA_VERSION, record::Document};

#[test]
fn unsupported_schema_is_neither_readable_nor_writable() {
    let mut record = document();
    let mut value = serde_json::to_value(&record).unwrap();
    value[SCHEMA_FIELD] = UNSUPPORTED_SCHEMA_VERSION.into();
    assert!(serde_json::from_value::<Document>(value.clone()).is_err());
    value.as_object_mut().unwrap().remove(INTENTS_FIELD);
    assert!(serde_json::from_value::<Document>(value).is_err());
    record.schema_version = UNSUPPORTED_SCHEMA_VERSION;
    assert!(serde_json::to_value(record).is_err());
}

#[test]
fn current_schema_requires_intents_and_rejects_unknown_versions_and_fields() {
    let current = document();
    let value = serde_json::to_value(&current).unwrap();
    assert_eq!(
        serde_json::from_value::<Document>(value.clone()).unwrap(),
        current
    );
    let mut absent = value.clone();
    absent.as_object_mut().unwrap().remove(INTENTS_FIELD);
    assert!(serde_json::from_value::<Document>(absent).is_err());
    let mut future = value.clone();
    future[SCHEMA_FIELD] = (JOURNAL_SCHEMA_VERSION + SCHEMA_STEP).into();
    assert!(serde_json::from_value::<Document>(future).is_err());
    let mut unknown = value;
    unknown[UNKNOWN_FIELD] = true.into();
    assert!(serde_json::from_value::<Document>(unknown).is_err());
}
