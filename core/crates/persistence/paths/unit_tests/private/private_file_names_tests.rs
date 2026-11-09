use super::*;
use std::ffi::{CStr, CString, OsStr};
use std::os::unix::ffi::OsStrExt;
use uuid::Uuid;

const TEST_TARGET: &str = "private-file-name-test-target";
const TEST_PREFIX: &str = ".private-file-name-test-prefix-";
const INVALID_SHORT_DIGEST: &str = "a";
const EXTRA_COMPONENT_SEPARATOR: &str = "-";
const GUARDIAN_RECORD_SUFFIX: &str = ".json";

#[test]
fn generated_private_file_names_are_storage_artifacts() {
    let target = CString::new(TEST_TARGET).unwrap();
    let active = active_name(&target, ATOMIC_FILE_PREFIX, ATOMIC_FILE_SUFFIX).unwrap();
    let names = [
        active,
        retired_name(&target).unwrap(),
        retained_isolation_name(&target).unwrap(),
        unlink_isolation_name(&target).unwrap(),
        staging_name(&target, TEST_PREFIX).unwrap(),
        quarantine_name(&target, TEST_PREFIX, &target).unwrap(),
    ];

    assert!(names.iter().all(|name| is_storage_artifact(os_str(name))));
}

#[test]
fn malformed_storage_names_remain_visible_to_callers() {
    let digest = "a".repeat(PRIVATE_FILE_NAME_HASH_HEX_LENGTH);
    let uppercase_digest = digest.to_uppercase();
    let retired = format!("{PRIVATE_FILE_RETIRED_PREFIX}{digest}");
    let malformed = [
        format!("{PRIVATE_FILE_RETIRED_PREFIX}{INVALID_SHORT_DIGEST}"),
        format!("{PRIVATE_FILE_RETIRED_PREFIX}{uppercase_digest}"),
        format!("{retired}{EXTRA_COMPONENT_SEPARATOR}{digest}"),
        format!("{PRIVATE_FILE_STAGING_PREFIX}{digest}"),
        format!("{PRIVATE_FILE_QUARANTINE_PREFIX}{digest}-{digest}"),
        format!("{ATOMIC_FILE_PREFIX}{}{ATOMIC_FILE_SUFFIX}", Uuid::new_v4()).to_uppercase(),
    ];

    assert!(
        malformed
            .iter()
            .all(|name| !is_storage_artifact(OsStr::new(name)))
    );
}

#[test]
fn canonical_guardian_record_name_is_not_a_storage_artifact() {
    let name = format!("{}{GUARDIAN_RECORD_SUFFIX}", Uuid::new_v4());

    assert!(!is_storage_artifact(OsStr::new(&name)));
}

#[test]
fn atomic_storage_artifacts_bind_to_their_exact_target() {
    let target = CString::new(format!("{}{GUARDIAN_RECORD_SUFFIX}", Uuid::new_v4())).unwrap();
    let other = CString::new(format!("{}{GUARDIAN_RECORD_SUFFIX}", Uuid::new_v4())).unwrap();
    let active = active_name(&target, ATOMIC_FILE_PREFIX, ATOMIC_FILE_SUFFIX).unwrap();
    let names = [
        active.clone(),
        retired_name(&target).unwrap(),
        retained_isolation_name(&target).unwrap(),
        unlink_isolation_name(&target).unwrap(),
        staging_name(&target, ATOMIC_FILE_PREFIX).unwrap(),
        quarantine_name(&target, ATOMIC_FILE_PREFIX, &active).unwrap(),
    ];

    assert!(
        names
            .iter()
            .all(|name| { is_atomic_storage_artifact_for(os_str(name), os_str(&target)).unwrap() })
    );
    assert!(
        names
            .iter()
            .all(|name| { !is_atomic_storage_artifact_for(os_str(name), os_str(&other)).unwrap() })
    );
}

fn os_str(value: &CStr) -> &OsStr {
    OsStr::from_bytes(value.to_bytes())
}
