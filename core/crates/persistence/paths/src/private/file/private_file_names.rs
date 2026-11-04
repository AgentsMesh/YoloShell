use crate::paths::constants::{
    ATOMIC_FILE_PREFIX, ATOMIC_FILE_SUFFIX, PRIVATE_FILE_NAME_HASH_BYTES,
    PRIVATE_FILE_NAME_HASH_HEX_LENGTH, PRIVATE_FILE_QUARANTINE_PREFIX,
    PRIVATE_FILE_RETAINED_ISOLATION_PREFIX, PRIVATE_FILE_RETIRED_PREFIX,
    PRIVATE_FILE_STAGING_PREFIX, PRIVATE_FILE_UNLINK_ISOLATION_PREFIX,
};
use anyhow::{Context, Result};
use std::ffi::{CStr, CString, OsStr};
use std::fmt::Write;
use std::os::unix::ffi::OsStrExt;

const SINGLE_DIGEST_COMPONENT_COUNT: usize = 1;
const STAGING_DIGEST_COMPONENT_COUNT: usize = 2;
const QUARANTINE_DIGEST_COMPONENT_COUNT: usize = 3;
const ARTIFACT_TARGET_NAME_DETAIL: &str = "storage artifact target name contains a null byte";

pub(super) fn active_name(target: &CStr, prefix: &str, suffix: &str) -> Result<CString> {
    let digest = russh::keys::HashAlg::Sha256.digest(target.to_bytes());
    let identifier = uuid::Uuid::from_slice(&digest[..PRIVATE_FILE_NAME_HASH_BYTES])?;
    CString::new(format!("{prefix}{identifier}{suffix}"))
        .context("active private staging name contains a null byte")
}

pub(super) fn staging_name(target: &CStr, prefix: &str) -> Result<CString> {
    CString::new(format!(
        "{PRIVATE_FILE_STAGING_PREFIX}{}-{}",
        digest_name(target.to_bytes()),
        digest_name(prefix.as_bytes())
    ))
    .context("private staging slot name contains a null byte")
}

pub(super) fn quarantine_name(target: &CStr, prefix: &str, source: &CStr) -> Result<CString> {
    CString::new(format!(
        "{PRIVATE_FILE_QUARANTINE_PREFIX}{}-{}-{}",
        digest_name(target.to_bytes()),
        digest_name(prefix.as_bytes()),
        digest_name(source.to_bytes())
    ))
    .context("private quarantine name contains a null byte")
}

pub(super) fn retired_name(target: &CStr) -> Result<CString> {
    CString::new(format!(
        "{PRIVATE_FILE_RETIRED_PREFIX}{}",
        digest_name(target.to_bytes())
    ))
    .context("retired private file name contains a null byte")
}

#[cfg(any(test, feature = "persistence-test-support"))]
pub(crate) fn retired_name_for_test(target: &CStr) -> Result<CString> {
    retired_name(target)
}

pub(super) fn retained_isolation_name(target: &CStr) -> Result<CString> {
    CString::new(format!(
        "{PRIVATE_FILE_RETAINED_ISOLATION_PREFIX}{}",
        digest_name(target.to_bytes())
    ))
    .context("retained isolation name contains a null byte")
}

pub(super) fn unlink_isolation_name(target: &CStr) -> Result<CString> {
    CString::new(format!(
        "{PRIVATE_FILE_UNLINK_ISOLATION_PREFIX}{}",
        digest_name(target.to_bytes())
    ))
    .context("unlink isolation name contains a null byte")
}

fn digest_name(value: &[u8]) -> String {
    let digest = russh::keys::HashAlg::Sha256.digest(value);
    let mut encoded = String::with_capacity(PRIVATE_FILE_NAME_HASH_HEX_LENGTH);
    for byte in digest.iter().take(PRIVATE_FILE_NAME_HASH_BYTES) {
        write!(&mut encoded, "{byte:02x}").expect("writing to String cannot fail");
    }
    encoded
}

pub fn is_storage_artifact(name: &OsStr) -> bool {
    let Some(value) = name.to_str() else {
        return false;
    };
    is_active(value)
        || has_digests(
            value,
            PRIVATE_FILE_RETIRED_PREFIX,
            SINGLE_DIGEST_COMPONENT_COUNT,
        )
        || has_digests(
            value,
            PRIVATE_FILE_RETAINED_ISOLATION_PREFIX,
            SINGLE_DIGEST_COMPONENT_COUNT,
        )
        || has_digests(
            value,
            PRIVATE_FILE_UNLINK_ISOLATION_PREFIX,
            SINGLE_DIGEST_COMPONENT_COUNT,
        )
        || has_digests(
            value,
            PRIVATE_FILE_STAGING_PREFIX,
            STAGING_DIGEST_COMPONENT_COUNT,
        )
        || has_digests(
            value,
            PRIVATE_FILE_QUARANTINE_PREFIX,
            QUARANTINE_DIGEST_COMPONENT_COUNT,
        )
}

pub fn is_atomic_storage_artifact_for(artifact: &OsStr, target: &OsStr) -> Result<bool> {
    let target = CString::new(target.as_bytes()).context(ARTIFACT_TARGET_NAME_DETAIL)?;
    let active = active_name(&target, ATOMIC_FILE_PREFIX, ATOMIC_FILE_SUFFIX)?;
    let retired = retired_name(&target)?;
    let retained = retained_isolation_name(&target)?;
    let unlink = unlink_isolation_name(&target)?;
    let staging = staging_name(&target, ATOMIC_FILE_PREFIX)?;
    let quarantine = quarantine_name(&target, ATOMIC_FILE_PREFIX, &active)?;
    Ok([active, retired, retained, unlink, staging, quarantine]
        .iter()
        .any(|expected| artifact.as_bytes() == expected.as_bytes()))
}

fn is_active(value: &str) -> bool {
    let Some(body) = value
        .strip_prefix(ATOMIC_FILE_PREFIX)
        .and_then(|value| value.strip_suffix(ATOMIC_FILE_SUFFIX))
    else {
        return false;
    };
    uuid::Uuid::parse_str(body).is_ok_and(|identifier| identifier.hyphenated().to_string() == body)
}

fn has_digests(value: &str, prefix: &str, count: usize) -> bool {
    let Some(body) = value.strip_prefix(prefix) else {
        return false;
    };
    let parts: Vec<_> = body.split('-').collect();
    parts.len() == count && parts.into_iter().all(is_digest)
}

fn is_digest(value: &str) -> bool {
    value.len() == PRIVATE_FILE_NAME_HASH_HEX_LENGTH
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../../unit_tests/private/private_file_names_tests.rs"]
mod tests;
