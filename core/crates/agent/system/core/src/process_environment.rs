#[cfg(any(target_os = "linux", test))]
use super::constants::PROCESS_ENVIRONMENT_READ_SENTINEL_BYTES;
use super::constants::{
    ENVIRONMENT_ASSIGNMENT_BYTE, MAX_PROCESS_ENVIRONMENT_BYTES, MAX_PROCESS_ENVIRONMENT_FIELDS,
    MAX_PROCESS_ENVIRONMENT_KEYS, MAX_PROCESS_ENVIRONMENT_RAW_BYTES,
    MAX_PROCESS_ENVIRONMENT_VALUE_BYTES, NUL_BYTE, process_environment_error,
};
use anyhow::{Context, Result, ensure};
use std::collections::{BTreeMap, BTreeSet};
#[cfg(any(target_os = "linux", test))]
use std::io::{self, Read};

#[cfg(test)]
pub fn read_selected_environment(
    reader: impl Read,
    keys: &[&str],
) -> Result<BTreeMap<String, String>> {
    let bytes = read_process_environment(reader).context(process_environment_error::READ)?;
    parse_selected_environment(&bytes, keys)
}

#[cfg(any(target_os = "linux", test))]
pub fn read_process_environment(reader: impl Read) -> io::Result<Vec<u8>> {
    let byte_limit = MAX_PROCESS_ENVIRONMENT_RAW_BYTES
        .checked_add(PROCESS_ENVIRONMENT_READ_SENTINEL_BYTES)
        .ok_or_else(|| io::Error::other(process_environment_error::RAW_LIMIT))?;
    let read_limit = u64::try_from(byte_limit)
        .map_err(|_| io::Error::other(process_environment_error::RAW_LIMIT))?;
    let mut bytes = Vec::with_capacity(byte_limit);
    reader
        .take(read_limit)
        .read_to_end(&mut bytes)
        .map(|_| bytes)
}

pub fn parse_selected_environment(bytes: &[u8], keys: &[&str]) -> Result<BTreeMap<String, String>> {
    ensure!(
        keys.len() <= MAX_PROCESS_ENVIRONMENT_KEYS,
        process_environment_error::KEY_LIMIT
    );
    ensure!(
        bytes.len() <= MAX_PROCESS_ENVIRONMENT_RAW_BYTES,
        process_environment_error::RAW_LIMIT
    );
    ensure!(
        bytes.is_empty() || bytes.last() == Some(&NUL_BYTE),
        process_environment_error::UNTERMINATED
    );
    let allowed: BTreeSet<&[u8]> = keys.iter().map(|key| key.as_bytes()).collect();
    ensure!(
        allowed.len() == keys.len(),
        process_environment_error::DUPLICATE_KEY
    );
    let mut selected = BTreeMap::new();
    let mut selected_bytes = 0_usize;
    for (index, field) in bytes
        .split(|byte| *byte == NUL_BYTE)
        .filter(|field| !field.is_empty())
        .enumerate()
    {
        ensure!(
            index < MAX_PROCESS_ENVIRONMENT_FIELDS,
            process_environment_error::FIELD_LIMIT
        );
        let Some(separator) = field
            .iter()
            .position(|byte| *byte == ENVIRONMENT_ASSIGNMENT_BYTE)
        else {
            ensure!(
                !allowed.contains(field),
                process_environment_error::MALFORMED
            );
            continue;
        };
        let (key, value) = (&field[..separator], &field[separator + 1..]);
        if !allowed.contains(key) {
            continue;
        }
        ensure!(
            value.len() <= MAX_PROCESS_ENVIRONMENT_VALUE_BYTES,
            process_environment_error::VALUE_LIMIT
        );
        let value = std::str::from_utf8(value).context(process_environment_error::VALUE_UTF8)?;
        selected_bytes = selected_bytes
            .checked_add(key.len())
            .and_then(|bytes| bytes.checked_add(value.len()))
            .context(process_environment_error::SELECTED_LIMIT)?;
        ensure!(
            selected_bytes <= MAX_PROCESS_ENVIRONMENT_BYTES,
            process_environment_error::SELECTED_LIMIT
        );
        let key = keys
            .iter()
            .copied()
            .find(|candidate| candidate.as_bytes() == key)
            .context(process_environment_error::MALFORMED)?;
        ensure!(
            selected.insert(key.to_owned(), value.to_owned()).is_none(),
            process_environment_error::DUPLICATE_KEY
        );
    }
    Ok(selected)
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../integration_tests/process_environment_tests.rs"]
mod tests;
