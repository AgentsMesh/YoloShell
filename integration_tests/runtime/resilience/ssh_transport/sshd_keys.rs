use super::super::constants::*;
use std::{
    path::{Path, PathBuf},
    process::Command,
};

pub(super) fn generate_key(path: &Path) {
    let status = Command::new(SSH_KEYGEN_PROGRAM)
        .args([
            KEYGEN_QUIET_ARGUMENT,
            KEYGEN_TYPE_ARGUMENT,
            KEYGEN_TYPE,
            KEYGEN_PASSPHRASE_ARGUMENT,
            EMPTY_PASSPHRASE,
            KEYGEN_OUTPUT_ARGUMENT,
        ])
        .arg(path)
        .status()
        .unwrap();
    assert!(status.success());
}

pub(super) fn public_key_path(private: &Path) -> PathBuf {
    PathBuf::from(format!("{}{PUBLIC_KEY_SUFFIX}", private.display()))
}

pub(super) fn public_key_fields(private: &Path) -> String {
    let public = std::fs::read_to_string(public_key_path(private)).unwrap();
    public
        .split_whitespace()
        .take(PUBLIC_KEY_FIELDS)
        .collect::<Vec<_>>()
        .join(PUBLIC_KEY_SEPARATOR)
}
