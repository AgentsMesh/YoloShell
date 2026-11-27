use super::acceptance_receipt_support::{bytes, open, parse, record, replace_once, try_signed};
use crate::constants::{ACCEPTANCE_ALGORITHM, ACCEPTANCE_AUTHORITY, MAX_ACCEPTANCE_RECEIPT_BYTES};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::fs::symlink;
use tempfile::tempdir;

#[test]
fn duplicate_envelope_field_is_rejected() {
    let mut mutated = bytes();
    let insertion = b",\"signature\":\"";
    let marker = b"\"\n";
    let position = mutated.len() - marker.len();
    mutated.splice(position..position, insertion.iter().copied());
    mutated.splice(
        position + insertion.len()..position + insertion.len(),
        b"x\"".iter().copied(),
    );
    assert!(parse(&mutated).is_err());
}

#[test]
fn unknown_field_is_rejected() {
    let mut mutated = bytes();
    let position = mutated.len() - 2;
    mutated.splice(position..position, b",\"unknown\":true".iter().copied());
    assert!(parse(&mutated).is_err());
}

#[test]
fn noncanonical_whitespace_is_rejected() {
    let mut mutated = bytes();
    mutated.insert(0, b' ');
    assert!(parse(&mutated).is_err());
}

#[test]
fn authority_mutation_is_rejected() {
    let mutated = replace_once(&bytes(), ACCEPTANCE_AUTHORITY, "foreign-authority");
    assert!(parse(&mutated).is_err());
}

#[test]
fn algorithm_mutation_is_rejected() {
    let mutated = replace_once(&bytes(), ACCEPTANCE_ALGORITHM, "sha256");
    assert!(parse(&mutated).is_err());
}

#[test]
fn failed_check_result_is_rejected() {
    let mutated = replace_once(&bytes(), "\"status\":\"passed\"", "\"status\":\"failed\"");
    assert!(parse(&mutated).is_err());
}

#[test]
fn missing_check_results_are_rejected() {
    let mutated = replace_once(&bytes(), "\"results\":[", "\"results\":[]");
    assert!(parse(&mutated).is_err());
}

#[test]
fn provider_reordering_is_rejected_by_shape() {
    let mut value = record();
    value.providers.reverse();
    assert!(try_signed(value).is_err());
}

#[test]
fn macos_below_minimum_is_rejected() {
    let mut value = record();
    value.desktop_os.version = "14.1".to_owned();
    assert!(try_signed(value).is_err());
}

#[test]
fn macos_version_suffix_and_extra_components_are_rejected() {
    let mut suffix = record();
    suffix.remote_os.version = "14.2-beta".to_owned();
    assert!(try_signed(suffix).is_err());

    let mut extra = record();
    extra.remote_os.version = "14.2.0.1".to_owned();
    assert!(try_signed(extra).is_err());
}

#[test]
fn macos_noncanonical_leading_zero_is_rejected() {
    let mut value = record();
    value.desktop_os.version = "14.02".to_owned();
    assert!(try_signed(value).is_err());
}

#[test]
fn newer_macos_patch_version_is_accepted() {
    let mut value = record();
    value.desktop_os.version = "14.2.1".to_owned();
    value.remote_os.version = "15.0".to_owned();
    assert!(try_signed(value).is_ok());
}

#[test]
fn invalid_signature_encoding_is_rejected() {
    let mutated = replace_once(&bytes(), "signature\":\"", "signature\":\"0");
    assert!(parse(&mutated).is_err());
}

#[test]
fn oversized_receipt_is_rejected_before_parse() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("oversized.json");
    std::fs::write(&path, vec![b'x'; MAX_ACCEPTANCE_RECEIPT_BYTES as usize + 1]).unwrap();
    std::fs::set_permissions(
        &path,
        std::fs::Permissions::from_mode(crate::constants::READ_ONLY_FILE_MODE),
    )
    .unwrap();
    assert_eq!(
        open(&path).err().unwrap().to_string(),
        crate::constants::RECEIPT_FILE_ERROR
    );
}

#[test]
fn symlink_receipt_is_rejected() {
    let directory = tempdir().unwrap();
    let target = directory.path().join("target.json");
    let link = directory.path().join("receipt.json");
    std::fs::write(&target, bytes()).unwrap();
    symlink(&target, &link).unwrap();
    assert!(open(&link).is_err());
}
