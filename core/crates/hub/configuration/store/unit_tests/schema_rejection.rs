use super::{constants::*, fixtures::*};

#[test]
fn missing_current_schema_fields_are_rejected_without_rewriting() {
    for removed in [
        vec![SCHEMA_FIELD],
        vec![LISTENER_INITIALIZED_FIELD],
        vec![SCHEMA_FIELD, LISTENER_INITIALIZED_FIELD],
    ] {
        let fixture = Fixture::new();
        let store = fixture.open(None).unwrap();
        let mut wire = serde_json::to_value(fixture.record()).unwrap();
        drop(store);
        for field in removed {
            wire.as_object_mut().unwrap().remove(field);
        }
        fixture.replace(&wire);
        let before = fixture.bytes();
        assert!(fixture.open(None).is_err());
        assert_eq!(fixture.bytes(), before);
    }
}

#[test]
fn unsupported_private_schema_is_rejected_without_reinitialization() {
    for schema in [
        crate::constants::PRIVATE_RECORD_SCHEMA - SCHEMA_STEP,
        crate::constants::PRIVATE_RECORD_SCHEMA + SCHEMA_STEP,
    ] {
        let fixture = Fixture::new();
        let store = fixture.open(None).unwrap();
        let mut record = fixture.record();
        drop(store);
        record.schema_version = schema;
        fixture.replace(&record);
        let before = fixture.bytes();
        assert_eq!(
            fixture.open(None).err().unwrap().to_string(),
            crate::constants::INVALID_PRIVATE_SCHEMA
        );
        assert_eq!(fixture.bytes(), before);
    }
}

#[test]
fn duplicate_current_record_fields_are_rejected_without_rewriting() {
    for field in [
        DIRECTORY_FIELD,
        STATE_FIELD,
        SCHEMA_FIELD,
        LISTENER_INITIALIZED_FIELD,
    ] {
        let fixture = Fixture::new();
        let store = fixture.open(None).unwrap();
        let record = serde_json::to_value(fixture.record()).unwrap();
        drop(store);
        let original = serde_json::to_string(&record).unwrap();
        let bytes = format!(
            "{{{}:{},{}",
            serde_json::to_string(field).unwrap(),
            record[field],
            original.strip_prefix('{').unwrap()
        )
        .into_bytes();
        std::fs::write(fixture.path.join(crate::constants::RECORD_FILE), &bytes).unwrap();
        assert!(fixture.open(None).is_err());
        assert_eq!(fixture.bytes(), bytes);
    }
}
