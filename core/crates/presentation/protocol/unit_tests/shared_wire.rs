use super::{fixture::*, *};
use serde_json::Value;
use uuid::Uuid;

#[test]
fn wire_roundtrips_share_topology_without_client_selection() {
    let request = update();
    let wire = serde_json::to_value(&request).unwrap();
    assert_eq!(
        serde_json::from_value::<LayoutUpdate>(wire.clone()).unwrap(),
        request
    );
    let document = request.apply(None).unwrap();
    let wire = serde_json::to_value(&document).unwrap();
    assert_eq!(
        serde_json::from_value::<SharedLayoutDocument>(wire.clone()).unwrap(),
        document
    );
    assert!(!wire.to_string().contains("focused"));
    assert!(!wire.to_string().contains("scroll"));
}

#[test]
fn retired_collection_field_never_aliases_tabs_in_requests_or_documents() {
    let request = update();
    for (current, document) in [
        (serde_json::to_value(&request).unwrap(), false),
        (serde_json::to_value(request.apply(None).unwrap()).unwrap(), true),
    ] {
        for retain_current in [false, true] {
            let mut changed = current.clone();
            let fields = changed.as_object_mut().unwrap();
            let tabs = if retain_current {
                fields[TABS_FIELD].clone()
            } else {
                fields.remove(TABS_FIELD).unwrap()
            };
            fields.insert(RETIRED_PANELS_FIELD.into(), tabs);
            if document {
                assert!(serde_json::from_value::<SharedLayoutDocument>(changed).is_err());
            } else {
                assert!(serde_json::from_value::<LayoutUpdate>(changed).is_err());
            }
        }
    }
}

#[test]
fn deserialization_rejects_noncanonical_nil_missing_and_unknown_identities() {
    let request = update();
    let original = serde_json::to_value(&request).unwrap();
    for path in [
        "/address/layout_id",
        "/request_id",
        "/tabs/0/id",
        "/tabs/0/root/pane/id",
    ] {
        for id in [
            Uuid::nil().to_string(),
            Uuid::new_v4().simple().to_string(),
            "not-an-id".into(),
            "AAAAAAAA-AAAA-4AAA-8AAA-AAAAAAAAAAAA".into(),
        ] {
            let mut value = original.clone();
            *value.pointer_mut(path).unwrap() = id.into();
            assert!(serde_json::from_value::<LayoutUpdate>(value).is_err());
        }
    }
    for path in ["", "/address", "/tabs/0", "/tabs/0/root/pane"] {
        let mut value = original.clone();
        value
            .pointer_mut(path)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert(FOREIGN_FIELD.into(), true.into());
        assert!(serde_json::from_value::<LayoutUpdate>(value).is_err());
    }
    let mut invalid = request;
    invalid.request_id = Uuid::nil();
    assert!(serde_json::to_value(invalid).is_err());
    let mut wire = serde_json::to_value(update().apply(None).unwrap()).unwrap();
    wire["revision"] = Value::from(0);
    assert!(serde_json::from_value::<SharedLayoutDocument>(wire.clone()).is_err());
    wire["revision"] = FIRST_REVISION.into();
    wire[FOREIGN_FIELD] = true.into();
    assert!(serde_json::from_value::<SharedLayoutDocument>(wire).is_err());
}
