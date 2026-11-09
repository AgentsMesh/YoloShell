use yolo_hub_delegation_test_support::Identity;
use yolo_hub_federation_settings::Configuration;
use yolo_hub_file_protocol::{FileRootConfiguration, FileRootFields, MAX_FILE_ROOTS};
use yolo_hub_identity::FileRootId;

const ROOT_NAME: &str = "Workspace";
const ROOT_PATH: &str = "/srv/project";

#[test]
fn file_root_settings_roundtrip_and_reject_duplicate_or_unbounded_registrations() {
    let local = Identity::new();
    let root = FileRootConfiguration::new(FileRootFields {
        root_id: FileRootId::new(),
        name: ROOT_NAME.into(),
        path: ROOT_PATH.into(),
    })
    .unwrap();
    let mut configuration = Configuration {
        file_roots: vec![root.clone()],
        ..Configuration::default()
    };
    configuration.validate(local.identity).unwrap();
    assert_eq!(
        serde_json::from_value::<Configuration>(serde_json::to_value(&configuration).unwrap())
            .unwrap(),
        configuration
    );
    configuration.file_roots.push(root.clone());
    assert!(configuration.validate(local.identity).is_err());
    configuration.file_roots = vec![root; MAX_FILE_ROOTS + 1];
    assert!(configuration.validate(local.identity).is_err());
}

#[test]
fn current_configuration_requires_file_roots_field() {
    let mut value = serde_json::to_value(Configuration::default()).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .remove(stringify!(file_roots));
    assert!(serde_json::from_value::<Configuration>(value).is_err());
}
