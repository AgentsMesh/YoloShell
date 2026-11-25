use crate::fixtures::*;
use yolo_hub_file_protocol::*;
use yolo_hub_identity::{FileRootEpoch, FileRootId};

#[test]
fn approved_root_metadata_roundtrips_without_becoming_a_filesystem_authorization() {
    let fields = root();
    let configuration = FileRootConfiguration::new(fields.clone()).unwrap();
    assert_eq!(configuration.fields(), &fields);
    assert_eq!(FileRootFields::from(configuration.clone()), fields);
    assert_eq!(
        FileRootConfiguration::try_from(fields).unwrap(),
        configuration
    );
    roundtrip(&configuration);
    let target = target();
    roundtrip(&target);
    roundtrip(&target.address);
    let fields = FileRootSummaryFields {
        target,
        name: ROOT_NAME.into(),
        path: ROOT_PATH.into(),
    };
    let summary = FileRootSummary::new(fields.clone()).unwrap();
    assert_eq!(summary.fields(), &fields);
    assert_eq!(FileRootSummaryFields::from(summary.clone()), fields);
    assert_eq!(FileRootSummary::try_from(fields.clone()).unwrap(), summary);
    roundtrip(&summary);
    for field in [NAME_FIELD, PATH_FIELD] {
        let mut invalid = serde_json::to_value(&summary).unwrap();
        invalid[field] = String::new().into();
        assert!(serde_json::from_value::<FileRootSummary>(invalid).is_err());
    }
    let mut invalid = serde_json::to_value(summary).unwrap();
    invalid[EXTRA_FIELD] = true.into();
    assert!(serde_json::from_value::<FileRootSummary>(invalid).is_err());
}

#[test]
fn roots_reject_filesystem_root_traversal_unbounded_and_control_character_names() {
    for path in [
        "", "/", "relative", "/a/../b", "/a/./b", "/a//b", "/a/", "/a\0b",
    ] {
        let mut fields = root();
        fields.path = path.into();
        assert!(FileRootConfiguration::new(fields.clone()).is_err());
        assert!(
            serde_json::from_value::<FileRootConfiguration>(serde_json::to_value(fields).unwrap())
                .is_err()
        );
    }
    let mut fields = root();
    fields.path = format!("/{}", SAMPLE_COMPONENT.repeat(MAX_FILE_PATH_BYTES));
    assert!(FileRootConfiguration::new(fields).is_err());
    for name in [
        String::new(),
        " ".into(),
        "a\nb".into(),
        SAMPLE_COMPONENT.repeat(MAX_FILE_ROOT_NAME_BYTES + 1),
    ] {
        let mut fields = root();
        fields.name = name;
        assert!(FileRootConfiguration::new(fields).is_err());
    }
    let mut fields = root();
    fields.name = SAMPLE_COMPONENT.repeat(MAX_FILE_ROOT_NAME_BYTES);
    assert!(FileRootConfiguration::new(fields).is_ok());
    let mut object = serde_json::to_value(root()).unwrap();
    object[EXTRA_FIELD] = true.into();
    assert!(serde_json::from_value::<FileRootConfiguration>(object).is_err());
}

#[test]
fn addresses_remain_stable_while_epochs_fence_a_reopened_scope() {
    let first = target();
    let second = FileRootTarget {
        epoch: FileRootEpoch::new(),
        ..first
    };
    assert_eq!(first.address, second.address);
    assert_ne!(first, second);
    let different = FileRootAddress {
        root_id: FileRootId::new(),
        ..first.address
    };
    assert_ne!(first.address, different);
    assert_eq!(
        std::collections::BTreeSet::from([first.address, first.address]).len(),
        1
    );
}
