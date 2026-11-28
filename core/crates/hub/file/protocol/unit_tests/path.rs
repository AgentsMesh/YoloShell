use crate::fixtures::*;
use yolo_hub_file_protocol::*;

#[test]
fn paths_are_relative_canonical_bounded_and_not_terminal_control_text() {
    let accepted = RelativeFilePath::new(RELATIVE.into()).unwrap();
    assert_eq!(accepted.as_str(), RELATIVE);
    assert_eq!(String::from(accepted.clone()), RELATIVE);
    assert_eq!(
        RelativeFilePath::try_from(RELATIVE.to_owned()).unwrap(),
        accepted
    );
    roundtrip(&accepted);
    for value in [
        "", "/", "/a", ".", "..", "a/../b", "a/./b", "a//b", "a/", "a\0b", "a\nb",
    ] {
        assert!(RelativeFilePath::new(value.into()).is_err(), "{value:?}");
        assert!(serde_json::from_value::<RelativeFilePath>(serde_json::json!(value)).is_err());
    }
    assert!(RelativeFilePath::new(SAMPLE_COMPONENT.repeat(MAX_FILE_COMPONENT_BYTES)).is_ok());
    assert!(RelativeFilePath::new(SAMPLE_COMPONENT.repeat(MAX_FILE_COMPONENT_BYTES + 1)).is_err());
    assert!(RelativeFilePath::new(vec![SAMPLE_COMPONENT; MAX_FILE_PATH_DEPTH].join("/")).is_ok());
    assert!(
        RelativeFilePath::new(vec![SAMPLE_COMPONENT; MAX_FILE_PATH_DEPTH + 1].join("/")).is_err()
    );
    assert!(RelativeFilePath::new(SAMPLE_COMPONENT.repeat(MAX_FILE_PATH_BYTES + 1)).is_err());
    assert!(serde_json::from_value::<RelativeFilePath>(serde_json::Value::Null).is_err());
}
