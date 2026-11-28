use super::*;
#[path = "../../test_support/unit/durable_file_version.rs"]
mod support;
use support::fixture;

const NEXT_COMPONENT: i64 = 1;

#[test]
fn content_version_ignores_only_change_time_while_unchanged_is_strict() {
    let before = fixture();
    let mut record = before.record;
    record.changed.nanoseconds += NEXT_COMPONENT;
    let after = DurableFileVersion::try_from(record).unwrap();
    assert!(before.same_object(after));
    assert!(before.same_content_version(after));
    assert!(!before.unchanged(after));
    assert_eq!(
        before.content_version_evidence(),
        after.content_version_evidence()
    );
    record.modified.nanoseconds += NEXT_COMPONENT;
    let modified = DurableFileVersion::try_from(record).unwrap();
    assert!(!before.same_content_version(modified));
    record = before.record;
    record.length += u64::try_from(NEXT_COMPONENT).unwrap();
    assert!(!before.same_content_version(DurableFileVersion::try_from(record).unwrap()));
}

#[test]
fn another_durable_object_is_never_the_same_file_version() {
    let before = fixture();
    let mut object = serde_json::to_value(before.record.object).unwrap();
    object["proof"]["macos"]["file_id"] = serde_json::json!(u64::MAX);
    let mut record = before.record;
    record.object = serde_json::from_value(object).unwrap();
    let changed = DurableFileVersion::try_from(record).unwrap();
    assert!(!before.same_object(changed));
    assert!(!before.same_content_version(changed));
    assert!(!before.unchanged(changed));
}

#[test]
fn file_version_roundtrips_and_rejects_bad_schema_kind_and_timestamps() {
    let valid = serde_json::to_value(fixture()).unwrap();
    assert_eq!(
        serde_json::from_value::<DurableFileVersion>(valid.clone()).unwrap(),
        fixture()
    );
    for field in ["modified", "changed"] {
        let mut wire = valid.clone();
        wire[field]["nanoseconds"] = serde_json::json!(NANOSECONDS_PER_SECOND);
        assert!(serde_json::from_value::<DurableFileVersion>(wire).is_err());
    }
    let mut wire = valid.clone();
    wire["schema"] = serde_json::json!(u16::MAX);
    assert!(serde_json::from_value::<DurableFileVersion>(wire).is_err());
    let mut wire = valid;
    wire["object"]["kind"] = serde_json::json!("directory");
    assert!(serde_json::from_value::<DurableFileVersion>(wire).is_err());
}
