#[path = "../test_support/constants.rs"]
mod constants;
#[path = "../test_support/fixtures.rs"]
mod fixtures;
use fixtures::*;
use serde_json::Value;
use yolo_hub_build_protocol::ServiceBuild;

#[test]
fn service_build_contains_only_three_strict_validated_images() {
    let expected = build();
    let bytes = serde_json::to_vec(&expected).unwrap();
    assert_eq!(
        serde_json::from_slice::<ServiceBuild>(&bytes).unwrap(),
        expected
    );
    let object = serde_json::to_value(&expected).unwrap();
    assert_eq!(object.as_object().unwrap().len(), IMAGE_FIELDS.len());
    for field in IMAGE_FIELDS {
        let mut missing = object.clone();
        missing.as_object_mut().unwrap().remove(*field);
        assert!(serde_json::from_value::<ServiceBuild>(missing).is_err());
        let mut extra = object.clone();
        extra[*field][UNKNOWN_FIELD] = UNKNOWN_VALUE.into();
        assert!(serde_json::from_value::<ServiceBuild>(extra).is_err());
    }
    let mut extra = object;
    extra[UNKNOWN_FIELD] = UNKNOWN_VALUE.into();
    assert!(serde_json::from_value::<ServiceBuild>(extra).is_err());
}

#[test]
fn no_service_can_supply_an_unvalidated_image() {
    for service in IMAGE_FIELDS {
        for (field, invalid) in [
            (SCHEMA_FIELD, Value::from(u16::MAX)),
            (DIGEST_FIELD, Value::from(DIGEST_CHARACTER)),
            (SIZE_FIELD, Value::from(0)),
        ] {
            let mut object = serde_json::to_value(build()).unwrap();
            object[*service][field] = invalid;
            assert!(serde_json::from_value::<ServiceBuild>(object).is_err());
        }
    }
}

#[test]
fn service_image_admission_rejects_each_mixed_service_generation() {
    let expected = build();
    assert!(expected.all_match(&expected.host));
    let replacement = yolo_build_image::BuildImage::new(
        constants::FOREIGN_DIGEST.repeat(yolo_build_image::SHA256_HEX_BYTES),
        constants::IMAGE_BYTES,
    )
    .unwrap();
    let mixed_builds = [
        ServiceBuild {
            host: replacement.clone(),
            ..expected.clone()
        },
        ServiceBuild {
            hub: replacement.clone(),
            ..expected.clone()
        },
        ServiceBuild {
            file_helper: replacement,
            ..expected.clone()
        },
    ];
    for mixed in mixed_builds {
        assert!(!mixed.all_match(&expected.host));
    }
}
