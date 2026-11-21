use serde_json::{Value, json};
use yolo_release_contracts::LINUX_RUNTIME_MANIFEST_SCHEMA;
use yolo_release_gate_test_support::constants::{
    ARTIFACT_SHA256_JSON_KEY, RUNTIME_MANIFEST_ARCHITECTURE_FIELD,
    RUNTIME_MANIFEST_ARCHITECTURE_JSON_KEY, RUNTIME_MANIFEST_BAZEL_DIGEST_JSON_KEY,
    RUNTIME_MANIFEST_BUNDLE_PATH_JSON_KEY, RUNTIME_MANIFEST_DUPLICATE_ARCHITECTURE_FIELD,
    RUNTIME_MANIFEST_DUPLICATE_ROOT_PREFIX, RUNTIME_MANIFEST_IMAGE_JSON_KEY,
    RUNTIME_MANIFEST_PLATFORM_JSON_KEY, RUNTIME_MANIFEST_ROOT_PREFIX, RUNTIME_MANIFEST_ROOT_SUFFIX,
    RUNTIME_MANIFEST_RUNTIMES_JSON_KEY, RUNTIME_MANIFEST_RUNTIMES_SEPARATOR,
    RUNTIME_MANIFEST_SCHEMA_JSON_KEY, RUNTIME_MANIFEST_UNKNOWN_JSON_KEY,
    RUNTIME_MANIFEST_VERSION_JSON_KEY, SOURCE_REVISION_JSON_KEY,
};
use yolo_release_gate_test_support::support::{REVISION, linux_runtimes};

pub const SCHEMA_KEY: &str = RUNTIME_MANIFEST_SCHEMA_JSON_KEY;
pub const REVISION_KEY: &str = SOURCE_REVISION_JSON_KEY;
pub const RUNTIMES_KEY: &str = RUNTIME_MANIFEST_RUNTIMES_JSON_KEY;
pub const ARCHITECTURE_KEY: &str = RUNTIME_MANIFEST_ARCHITECTURE_JSON_KEY;
pub const PLATFORM_KEY: &str = RUNTIME_MANIFEST_PLATFORM_JSON_KEY;
pub const IMAGE_KEY: &str = RUNTIME_MANIFEST_IMAGE_JSON_KEY;
pub const BAZEL_DIGEST_KEY: &str = RUNTIME_MANIFEST_BAZEL_DIGEST_JSON_KEY;
pub const VERSION_KEY: &str = RUNTIME_MANIFEST_VERSION_JSON_KEY;
pub const ARTIFACT_DIGEST_KEY: &str = ARTIFACT_SHA256_JSON_KEY;
pub const BUNDLE_PATH_KEY: &str = RUNTIME_MANIFEST_BUNDLE_PATH_JSON_KEY;
pub const DESCRIPTOR_KEY: &str =
    yolo_release_gate_test_support::constants::RUNTIME_MANIFEST_DESCRIPTOR_JSON_KEY;
pub const TARGET_KEY: &str = yolo_release_gate_test_support::constants::BUNDLE_TARGET_JSON_KEY;
pub const BUILD_ID_KEY: &str = yolo_release_gate_test_support::constants::BUNDLE_BUILD_ID_JSON_KEY;
pub const ARTIFACTS_KEY: &str =
    yolo_release_gate_test_support::constants::BUNDLE_ARTIFACTS_JSON_KEY;
pub const UNKNOWN_KEY: &str = RUNTIME_MANIFEST_UNKNOWN_JSON_KEY;
pub const FIRST_RUNTIME_INDEX: usize = 0;
pub const SECOND_RUNTIME_INDEX: usize = 1;
const FIRST_MATCH_COUNT: usize = 1;

pub fn valid_value() -> Value {
    json!({
        SCHEMA_KEY: LINUX_RUNTIME_MANIFEST_SCHEMA,
        REVISION_KEY: REVISION,
        RUNTIMES_KEY: linux_runtimes(),
    })
}

pub fn valid_bytes() -> Vec<u8> {
    serde_json::to_vec(&valid_value()).unwrap()
}

pub fn assert_rejected(bytes: &[u8]) {
    assert!(
        crate::runtime_manifest_audit::audit_manifest_bytes(bytes, REVISION, &linux_runtimes())
            .is_err()
    );
}

pub fn assert_value_rejected(value: &Value) {
    assert_rejected(&serde_json::to_vec(value).unwrap());
}

pub fn duplicate_root_bytes() -> Vec<u8> {
    let runtimes = serde_json::to_string(&linux_runtimes()).unwrap();
    [
        RUNTIME_MANIFEST_DUPLICATE_ROOT_PREFIX,
        REVISION,
        RUNTIME_MANIFEST_RUNTIMES_SEPARATOR,
        &runtimes,
        RUNTIME_MANIFEST_ROOT_SUFFIX,
    ]
    .concat()
    .into_bytes()
}

pub fn duplicate_runtime_bytes() -> Vec<u8> {
    let runtimes = serde_json::to_string(&linux_runtimes()).unwrap();
    let duplicated = runtimes.replacen(
        RUNTIME_MANIFEST_ARCHITECTURE_FIELD,
        RUNTIME_MANIFEST_DUPLICATE_ARCHITECTURE_FIELD,
        FIRST_MATCH_COUNT,
    );
    [
        RUNTIME_MANIFEST_ROOT_PREFIX,
        REVISION,
        RUNTIME_MANIFEST_RUNTIMES_SEPARATOR,
        &duplicated,
        RUNTIME_MANIFEST_ROOT_SUFFIX,
    ]
    .concat()
    .into_bytes()
}
