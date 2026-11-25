use super::{constants::*, fixtures::*};
use serde_json::{Value, json};
use yolo_distribution_bundle::*;

#[test]
fn json_key_order_and_whitespace_do_not_change_the_canonical_build_identity() {
    let bundle = descriptor();
    let direct = serde_json::to_vec(&bundle).unwrap();
    let reordered = serde_json::to_vec_pretty(&serde_json::to_value(&bundle).unwrap()).unwrap();
    assert_ne!(direct, reordered);
    let decoded: BundleDescriptor = serde_json::from_slice(&reordered).unwrap();
    assert_eq!(decoded, bundle);
    assert_eq!(decoded.build_id(), GOLDEN_BUILD_ID);
}

#[test]
fn retired_multi_executable_manifests_and_roles_are_rejected_without_aliases() {
    let original = serde_json::to_value(descriptor()).unwrap();
    let mut old_schema = original.clone();
    old_schema[SCHEMA_KEY] = json!(RETIRED_SCHEMA);
    reject(old_schema);
    let artifact = serde_json::to_value(artifacts().runtime).unwrap();
    for role in RETIRED_ROLES {
        assert!(serde_json::from_value::<ArtifactRole>(json!(role)).is_err());
        let mut replaced = original.clone();
        replaced[ARTIFACTS_KEY] = json!({ role: artifact.clone() });
        reject(replaced);
        let mut additional = original.clone();
        additional[ARTIFACTS_KEY][role] = artifact.clone();
        reject(additional);
    }
    let mut old_manifest = original;
    old_manifest[ARTIFACTS_KEY] = RETIRED_ROLES
        .map(|role| (role.to_owned(), artifact.clone()))
        .into_iter()
        .collect::<serde_json::Map<String, Value>>()
        .into();
    reject(old_manifest.clone());
    old_manifest[SCHEMA_KEY] = json!(RETIRED_SCHEMA);
    reject(old_manifest);
}

#[test]
fn missing_or_unknown_fields_are_rejected_at_every_contract_layer() {
    let original = serde_json::to_value(descriptor()).unwrap();
    for path in contract_objects() {
        let object = locate(&mut original.clone(), &path)
            .as_object()
            .unwrap()
            .clone();
        for key in object.keys() {
            let mut changed = original.clone();
            locate(&mut changed, &path)
                .as_object_mut()
                .unwrap()
                .remove(key);
            reject(changed);
        }
        let mut changed = original.clone();
        locate(&mut changed, &path)
            .as_object_mut()
            .unwrap()
            .insert(UNRECOGNIZED_KEY.into(), json!(true));
        reject(changed);
    }
    let mut arbitrary_path = original.clone();
    arbitrary_path[ARTIFACTS_KEY][ArtifactRole::Runtime.as_str()][PATH_KEY] = json!(PATH_ESCAPE);
    reject(arbitrary_path);
    let mut extra_role = original;
    extra_role[ARTIFACTS_KEY][UNRECOGNIZED_ROLE] =
        serde_json::to_value(artifacts().runtime).unwrap();
    reject(extra_role);
}

#[test]
fn identical_duplicate_fields_are_rejected_even_when_a_last_wins_parser_would_accept_them() {
    let original = serde_json::to_value(descriptor()).unwrap();
    let encoded = serde_json::to_string(&original).unwrap();
    for path in contract_objects() {
        let object = locate(&mut original.clone(), &path)
            .as_object()
            .unwrap()
            .clone();
        for (key, value) in object {
            let field = format!("{}:{}", serde_json::to_string(&key).unwrap(), value);
            assert!(encoded.contains(&field));
            let duplicate = encoded.replacen(&field, &format!("{field},{field}"), 1);
            assert!(serde_json::from_str::<BundleDescriptor>(&duplicate).is_err());
        }
    }
}

#[test]
fn unsupported_targets_invalid_metadata_and_forged_build_ids_are_rejected() {
    let original = serde_json::to_value(descriptor()).unwrap();
    for (key, value) in [
        (SCHEMA_KEY, json!(BUNDLE_SCHEMA_VERSION + 1)),
        (REVISION_KEY, json!(PATH_ESCAPE)),
        (VERSION_KEY, json!(INVALID_VERSION)),
        (BUILD_KEY, json!(RUNTIME_DIGEST)),
    ] {
        let mut changed = original.clone();
        changed[key] = value;
        reject(changed);
    }
    for (key, value) in [
        (PLATFORM_KEY, UNRECOGNIZED_PLATFORM),
        (ARCHITECTURE_KEY, UNRECOGNIZED_ARCHITECTURE),
    ] {
        let mut changed = original.clone();
        changed[TARGET_KEY][key] = json!(value);
        reject(changed);
    }
    for role in ArtifactRole::ALL {
        for (key, value) in [(SIZE_KEY, json!(0)), (DIGEST_KEY, json!(INVALID_DIGEST))] {
            let mut changed = original.clone();
            changed[ARTIFACTS_KEY][role.as_str()][key] = value;
            reject(changed);
        }
    }
    assert!(serde_json::from_value::<ArtifactRole>(json!(UNRECOGNIZED_ROLE)).is_err());
    for role in ArtifactRole::ALL {
        assert_eq!(
            serde_json::from_value::<ArtifactRole>(json!(role.as_str())).unwrap(),
            role
        );
    }
}

fn contract_objects() -> Vec<Vec<&'static str>> {
    let mut paths = vec![vec![], vec![TARGET_KEY], vec![ARTIFACTS_KEY]];
    paths.extend(ArtifactRole::ALL.map(|role| vec![ARTIFACTS_KEY, role.as_str()]));
    paths
}

fn locate<'a>(value: &'a mut Value, path: &[&str]) -> &'a mut Value {
    path.iter().fold(value, |value, field| &mut value[*field])
}

fn reject(value: Value) {
    assert!(serde_json::from_value::<BundleDescriptor>(value).is_err());
}
