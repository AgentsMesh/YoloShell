use super::super::constants::*;
use anyhow::{Context, Result};
use std::ffi::{CStr, CString};

pub(in crate::remote_broker) fn entry_name(value: impl AsRef<[u8]>) -> Result<CString> {
    CString::new(value.as_ref()).context("broker generation name contains a null byte")
}

pub(super) fn pending_payload_name(slot: &CStr) -> Result<CString> {
    entry_name(format!(
        "{}{PENDING_GENERATION_PAYLOAD_SUFFIX}",
        slot.to_string_lossy()
    ))
}

pub(super) fn generation_file_names() -> Vec<CString> {
    let mut names = vec![
        constant_name(GENERATION_CONFIG_FILE),
        constant_name(GENERATION_KNOWN_HOSTS_FILE),
        constant_name(GENERATION_ARTIFACT_FILE),
    ];
    names.extend((0..MAX_IDENTITY_FILES).map(|index| {
        entry_name(format!("{GENERATION_IDENTITY_PREFIX}{index}"))
            .expect("constant identity file name is valid")
    }));
    names
}

pub(super) fn is_generation_file_name(name: &CStr) -> bool {
    generation_file_names()
        .iter()
        .any(|expected| expected.as_c_str() == name)
}

fn constant_name(value: &str) -> CString {
    entry_name(value).expect("constant generation file name is valid")
}
