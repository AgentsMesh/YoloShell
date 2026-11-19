use super::{constants::*, fixture::Fixture};
use crate::{Selection, SelectionFields, constants::*};
use serde_json::{Value, json};
use uuid::Uuid;

#[test]
fn strict_selection_roundtrip_and_identity_validation() {
    let fixture = Fixture::new();
    let (_, descriptor) = fixture.stage(FIRST_FILL);
    let operation = Uuid::new_v4();
    let selection = Selection::next(None, operation, descriptor.build_id()).unwrap();
    assert_eq!(selection.revision(), FIRST_REVISION);
    assert_eq!(selection.operation_id(), operation);
    assert_eq!(selection.build_id(), descriptor.build_id());
    let original = serde_json::to_value(&selection).unwrap();
    assert_eq!(
        serde_json::from_value::<Selection>(original.clone()).unwrap(),
        selection
    );
    let fields: SelectionFields = selection.clone().into();
    assert_eq!(selection.fields(), &fields);
    for (key, value) in [
        (SCHEMA_KEY, json!(SELECTION_SCHEMA_VERSION + 1)),
        (REVISION_KEY, json!(0)),
        (OPERATION_KEY, json!(Uuid::nil())),
        (BUILD_KEY, json!(INVALID_BUILD)),
        (BUILD_KEY, json!(descriptor.build_id().to_uppercase())),
    ] {
        let mut changed = original.clone();
        changed[key] = value;
        assert!(serde_json::from_value::<Selection>(changed).is_err());
    }
    for key in [SCHEMA_KEY, REVISION_KEY, OPERATION_KEY, BUILD_KEY] {
        let mut changed = original.clone();
        changed.as_object_mut().unwrap().remove(key);
        assert!(serde_json::from_value::<Selection>(changed).is_err());
    }
    let encoded = serde_json::to_string(&original).unwrap();
    for (key, value) in original.as_object().unwrap() {
        let field = format!("{}:{}", serde_json::to_string(key).unwrap(), value);
        let duplicate = encoded.replacen(&field, &format!("{field},{field}"), 1);
        assert!(serde_json::from_str::<Selection>(&duplicate).is_err());
    }
    let mut extra = original;
    extra[EXTRA_KEY] = Value::Bool(true);
    assert!(serde_json::from_value::<Selection>(extra).is_err());
    let mut fields = fields;
    fields.revision = u64::MAX;
    let exhausted = Selection::try_from(fields).unwrap();
    assert!(Selection::next(Some(&exhausted), Uuid::new_v4(), descriptor.build_id()).is_err());
}

#[test]
fn nil_operation_and_stage_are_rejected_without_a_selector() {
    let fixture = Fixture::new();
    let store = fixture.store();
    let (stage, descriptor) = fixture.stage(FIRST_FILL);
    assert!(store.stage_path(Uuid::nil()).is_err());
    assert!(
        store
            .publish(stage, &descriptor, None, Uuid::nil())
            .is_err()
    );
    assert!(
        store
            .publish(Uuid::nil(), &descriptor, None, Uuid::new_v4())
            .is_err()
    );
    assert_eq!(store.inspect().unwrap(), None);
    assert_eq!(store.inspect_release(&descriptor).unwrap(), None);
}
