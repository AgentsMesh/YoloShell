use super::runtime_manifest_fixture::*;
use super::support::{OTHER_REVISION, REVISION, linux_runtimes};
use crate::runtime_manifest_audit::MAX_RUNTIME_MANIFEST_BYTES;
use serde_json::{Value, json};
use yolo_release_contracts::LINUX_RUNTIME_MANIFEST_SCHEMA;
use yolo_release_foundation::validation::SHA256_HEX_LENGTH;

const OVERFLOW_SENTINEL_BYTES: usize = 1;

#[test]
fn strict_manifest_accepts_exact_sealed_evidence() {
    crate::runtime_manifest_audit::audit_manifest_bytes(
        &valid_bytes(),
        REVISION,
        &linux_runtimes(),
    )
    .unwrap();
}

#[test]
fn raw_duplicate_root_and_runtime_keys_are_rejected() {
    assert_rejected(&duplicate_root_bytes());
    assert_rejected(&duplicate_runtime_bytes());
}

#[test]
fn unknown_root_and_runtime_keys_are_rejected() {
    let mut root = valid_value();
    root.as_object_mut()
        .unwrap()
        .insert(UNKNOWN_KEY.into(), json!(1));
    assert_value_rejected(&root);

    let mut runtime = valid_value();
    runtime[RUNTIMES_KEY][FIRST_RUNTIME_INDEX][UNKNOWN_KEY] = json!(1);
    assert_value_rejected(&runtime);
}

#[test]
fn missing_and_wrongly_typed_root_fields_are_rejected() {
    for key in [SCHEMA_KEY, REVISION_KEY, RUNTIMES_KEY] {
        let mut value = valid_value();
        value.as_object_mut().unwrap().remove(key);
        assert_value_rejected(&value);
    }
    let wrong_values = [
        (SCHEMA_KEY, json!("1")),
        (REVISION_KEY, json!(1)),
        (RUNTIMES_KEY, json!({})),
    ];
    for (key, wrong) in wrong_values {
        let mut value = valid_value();
        value[key] = wrong;
        assert_value_rejected(&value);
    }
}

#[test]
fn every_runtime_field_is_required_and_strongly_typed() {
    let keys = [IMAGE_KEY, BAZEL_DIGEST_KEY, DESCRIPTOR_KEY, BUNDLE_PATH_KEY];
    for key in keys {
        let mut missing = valid_value();
        missing[RUNTIMES_KEY][FIRST_RUNTIME_INDEX]
            .as_object_mut()
            .unwrap()
            .remove(key);
        assert_value_rejected(&missing);
        let mut wrong_type = valid_value();
        wrong_type[RUNTIMES_KEY][FIRST_RUNTIME_INDEX][key] = json!(1);
        assert_value_rejected(&wrong_type);
    }
}

#[test]
fn schema_revision_and_runtime_count_are_exact() {
    let mut legacy = valid_value();
    legacy[SCHEMA_KEY] = json!(LINUX_RUNTIME_MANIFEST_SCHEMA - 1);
    assert_value_rejected(&legacy);
    let mut schema = valid_value();
    schema[SCHEMA_KEY] = json!(LINUX_RUNTIME_MANIFEST_SCHEMA + 1);
    assert_value_rejected(&schema);

    let mut revision = valid_value();
    revision[REVISION_KEY] = json!(OTHER_REVISION);
    assert_value_rejected(&revision);
    let mut uppercase = valid_value();
    uppercase[REVISION_KEY] = json!(REVISION.to_ascii_uppercase());
    assert_value_rejected(&uppercase);

    let mut missing = valid_value();
    missing[RUNTIMES_KEY].as_array_mut().unwrap().pop();
    assert_value_rejected(&missing);
    let mut extra = valid_value();
    let duplicate = extra[RUNTIMES_KEY][FIRST_RUNTIME_INDEX].clone();
    extra[RUNTIMES_KEY].as_array_mut().unwrap().push(duplicate);
    assert_value_rejected(&extra);
}

#[test]
fn every_runtime_value_is_bound_to_independent_evidence() {
    let replacements: &[(&str, Value)] = &[
        (IMAGE_KEY, json!("other")),
        (DESCRIPTOR_KEY, json!("other")),
        (BUNDLE_PATH_KEY, json!("other")),
        (BAZEL_DIGEST_KEY, json!("0".repeat(SHA256_HEX_LENGTH))),
    ];
    for (key, replacement) in replacements {
        let mut value = valid_value();
        value[RUNTIMES_KEY][FIRST_RUNTIME_INDEX][key] = replacement.clone();
        assert_value_rejected(&value);
    }
}

#[test]
fn architecture_and_bundle_path_aliases_are_rejected() {
    for key in [DESCRIPTOR_KEY, BUNDLE_PATH_KEY] {
        let mut value = valid_value();
        let alias = value[RUNTIMES_KEY][FIRST_RUNTIME_INDEX][key].clone();
        value[RUNTIMES_KEY][SECOND_RUNTIME_INDEX][key] = alias;
        assert_value_rejected(&value);
    }
}

#[test]
fn bundle_targets_build_id_and_every_role_are_required_and_bound() {
    for role in yolo_distribution_bundle::ArtifactRole::ALL {
        let mut value = valid_value();
        value[RUNTIMES_KEY][FIRST_RUNTIME_INDEX][DESCRIPTOR_KEY][ARTIFACTS_KEY]
            .as_object_mut()
            .unwrap()
            .remove(role.as_str());
        assert_value_rejected(&value);
    }
    for key in [ARCHITECTURE_KEY, PLATFORM_KEY] {
        let mut value = valid_value();
        value[RUNTIMES_KEY][FIRST_RUNTIME_INDEX][DESCRIPTOR_KEY][TARGET_KEY][key] = json!("other");
        assert_value_rejected(&value);
    }
    let mut value = valid_value();
    value[RUNTIMES_KEY][FIRST_RUNTIME_INDEX][DESCRIPTOR_KEY][BUILD_ID_KEY] =
        json!("0".repeat(SHA256_HEX_LENGTH));
    assert_value_rejected(&value);
}

#[test]
fn manifest_byte_budget_is_enforced_before_json_parsing() {
    let oversized = MAX_RUNTIME_MANIFEST_BYTES as usize + OVERFLOW_SENTINEL_BYTES;
    assert_rejected(&vec![b' '; oversized]);
}

#[test]
fn retired_three_executable_manifests_cannot_supply_the_single_runtime_artifact() {
    use yolo_release_gate_test_support::constants::{
        RETIRED_BUNDLE_ARTIFACT_ROLES, RETIRED_BUNDLE_SCHEMA_VERSION,
    };
    for retain_runtime in [false, true] {
        let mut value = valid_value();
        let artifacts =
            &mut value[RUNTIMES_KEY][FIRST_RUNTIME_INDEX][DESCRIPTOR_KEY][ARTIFACTS_KEY];
        let runtime = yolo_distribution_bundle::ArtifactRole::Runtime.as_str();
        let component = artifacts[runtime].clone();
        if !retain_runtime {
            artifacts.as_object_mut().unwrap().remove(runtime);
        }
        for role in RETIRED_BUNDLE_ARTIFACT_ROLES {
            artifacts[*role] = component.clone();
        }
        assert_value_rejected(&value);
    }
    let mut value = valid_value();
    value[RUNTIMES_KEY][FIRST_RUNTIME_INDEX][DESCRIPTOR_KEY][SCHEMA_KEY] =
        json!(RETIRED_BUNDLE_SCHEMA_VERSION);
    assert_value_rejected(&value);
}
