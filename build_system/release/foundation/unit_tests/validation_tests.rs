use crate::validation::{
    GIT_SHA1_HEX_LENGTH, GIT_SHA256_HEX_LENGTH, validate_revision, validate_sha256,
};

const LOWERCASE_HEX: &str = "a";
const UPPERCASE_HEX: &str = "A";
const NON_HEX: &str = "g";

#[test]
fn accepts_supported_lowercase_git_object_identifiers() {
    assert!(validate_revision(&LOWERCASE_HEX.repeat(GIT_SHA1_HEX_LENGTH)).is_ok());
    assert!(validate_revision(&LOWERCASE_HEX.repeat(GIT_SHA256_HEX_LENGTH)).is_ok());
}

#[test]
fn rejects_wrong_length_uppercase_and_non_hex_identifiers() {
    assert!(validate_revision(&LOWERCASE_HEX.repeat(GIT_SHA1_HEX_LENGTH - 1)).is_err());
    assert!(validate_revision(&UPPERCASE_HEX.repeat(GIT_SHA1_HEX_LENGTH)).is_err());
    assert!(validate_revision(&NON_HEX.repeat(GIT_SHA1_HEX_LENGTH)).is_err());
}

#[test]
fn sha256_requires_exact_lowercase_hex_encoding() {
    assert!(validate_sha256(&LOWERCASE_HEX.repeat(GIT_SHA256_HEX_LENGTH)).is_ok());
    assert!(validate_sha256(&LOWERCASE_HEX.repeat(GIT_SHA1_HEX_LENGTH)).is_err());
    assert!(validate_sha256(&UPPERCASE_HEX.repeat(GIT_SHA256_HEX_LENGTH)).is_err());
}
