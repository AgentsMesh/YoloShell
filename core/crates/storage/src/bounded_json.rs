use crate::constants::{
    DEFAULT_STATE_FILE_NAME, DIRECTORY_MODE_PRIVATE, DUPLICATE_PATH_SEPARATOR, FILE_MODE_PRIVATE,
    READ_BUFFER_CAPACITY_BYTES, READ_LIMIT_SENTINEL_BYTES, TEMP_FILE_PREFIX,
};
use crate::error::StorageError;
use serde::{Serialize, de::DeserializeOwned};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JsonLimits {
    pub max_bytes: usize,
}

impl Default for JsonLimits {
    fn default() -> Self {
        Self {
            max_bytes: crate::DEFAULT_JSON_MAX_BYTES,
        }
    }
}

pub fn encode_json<T: Serialize>(value: &T, limits: JsonLimits) -> Result<Vec<u8>, StorageError> {
    let bytes = serde_json::to_vec(value)?;
    (bytes.len() <= limits.max_bytes)
        .then_some(bytes)
        .ok_or(StorageError::Limit)
}

pub fn decode_json<T: DeserializeOwned>(
    bytes: &[u8],
    limits: JsonLimits,
) -> Result<T, StorageError> {
    (bytes.len() <= limits.max_bytes)
        .then_some(())
        .ok_or(StorageError::Limit)?;
    Ok(serde_json::from_slice(bytes)?)
}

pub fn atomic_write_json<T: Serialize>(
    path: &Path,
    value: &T,
    limits: JsonLimits,
) -> Result<(), StorageError> {
    validate_path(path)?;
    let bytes = encode_json(value, limits)?;
    let parent = path.parent().ok_or(StorageError::InvalidPath)?;
    fs::create_dir_all(parent)?;
    set_private_directory(parent)?;
    let temporary = tempfile_path(path);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    if let Err(error) = write_private_file(&mut file, &bytes) {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    drop(file);
    if let Err(error) = fs::rename(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(error.into());
    }
    File::open(parent)?.sync_all()?;
    Ok(())
}

pub fn read_json<T: DeserializeOwned>(path: &Path, limits: JsonLimits) -> Result<T, StorageError> {
    validate_path(path)?;
    let mut file = File::open(path)?;
    let mut bytes = Vec::with_capacity(limits.max_bytes.min(READ_BUFFER_CAPACITY_BYTES));
    std::io::Read::by_ref(&mut file)
        .take((limits.max_bytes as u64).saturating_add(READ_LIMIT_SENTINEL_BYTES))
        .read_to_end(&mut bytes)?;
    decode_json(&bytes, limits)
}

fn tempfile_path(path: &Path) -> std::path::PathBuf {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(DEFAULT_STATE_FILE_NAME);
    path.with_file_name(format!(
        "{TEMP_FILE_PREFIX}{name}-{}",
        Uuid::new_v4().hyphenated()
    ))
}

fn validate_path(path: &Path) -> Result<(), StorageError> {
    if !path.is_absolute() || path.to_string_lossy().contains(DUPLICATE_PATH_SEPARATOR) {
        return Err(StorageError::InvalidPath);
    }
    let normalized = path.components().try_fold(
        std::path::PathBuf::from(crate::constants::ROOT_DIRECTORY),
        |mut output, component| match component {
            std::path::Component::RootDir => Ok(output),
            std::path::Component::Normal(value) => {
                output.push(value);
                Ok(output)
            }
            _ => Err(StorageError::InvalidPath),
        },
    )?;
    if normalized != path {
        return Err(StorageError::InvalidPath);
    }
    Ok(())
}

fn write_private_file(file: &mut File, bytes: &[u8]) -> Result<(), StorageError> {
    set_private_file(file)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

#[cfg(unix)]
fn set_private_file(file: &File) -> Result<(), StorageError> {
    use std::os::unix::fs::PermissionsExt;
    file.set_permissions(fs::Permissions::from_mode(FILE_MODE_PRIVATE))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_private_file(_file: &File) -> Result<(), StorageError> {
    Ok(())
}

#[cfg(unix)]
fn set_private_directory(path: &Path) -> Result<(), StorageError> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(DIRECTORY_MODE_PRIVATE))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_private_directory(_path: &Path) -> Result<(), StorageError> {
    Ok(())
}
