use crate::constants::joined;
use yolo_architecture_test_support::Fixture;

#[test]
fn accepts_unavoidable_serde_protocol_metadata() {
    let fixture = Fixture::new();
    let tag = joined(&["hook_event", "_name"]);
    fixture.write(
        "core/src/constants.rs",
        format!("pub const EVENT_TAG: &str = \"{tag}\";\n"),
    );
    fixture.write(
        "core/src/model.rs",
        format!("#[derive(serde::Serialize)]\n#[serde(tag = \"{tag}\")]\nenum Event {{ Ready }}\n"),
    );
    let violations = crate::inspect(fixture.root()).unwrap();
    assert!(violations.is_empty(), "{violations:?}");
}

#[test]
fn rejects_unregistered_serde_protocol_tag() {
    let fixture = Fixture::new();
    fixture.write(
        "core/src/model.rs",
        "#[derive(serde::Serialize)]\n#[serde(tag = \"unregistered_event_name\")]\nenum Event { Ready }\n",
    );
    let violations = crate::inspect(fixture.root()).unwrap();
    assert_eq!(violations.len(), 1);
}
