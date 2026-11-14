use super::constants::*;
use super::security_file::{effective_uid, inspect_secure_file, verify_secure_file_stable};
use anyhow::{Context, Result, ensure};
use std::fs;
use std::io::Read;
use std::path::{Component, Path};

pub(super) fn prepare_private_directory(
    path: &Path,
) -> Result<crate::paths::directory::PrivateDirectory> {
    validate_absolute(path, BROKER_STATE_DIRECTORY_LABEL)?;
    crate::paths::directory::PrivateDirectory::ensure(path)
}

pub(super) fn read_private_file(path: &Path, label: &str) -> Result<Vec<u8>> {
    let limit = match label {
        CONFIG_LABEL => MAX_CONFIG_FILE_BYTES,
        IDENTITY_LABEL => MAX_IDENTITY_FILE_BYTES,
        _ => MAX_CONFIG_FILE_BYTES,
    };
    read_validated(path, label, PRIVATE_FILE_MODE_MASK, false, limit)
}

pub(super) fn read_trusted_file(path: &Path, label: &str) -> Result<Vec<u8>> {
    let limit = match label {
        KNOWN_HOSTS_LABEL => MAX_KNOWN_HOSTS_BYTES,
        ARTIFACT_LABEL => MAX_ARTIFACT_FILE_BYTES,
        _ => MAX_CONFIG_FILE_BYTES,
    };
    read_validated(path, label, READ_ONLY_FILE_MODE_MASK, true, limit)
}

pub(super) fn read_identity(path: &Path) -> Result<Vec<u8>> {
    read_private_file(path, IDENTITY_LABEL)
}

pub(super) fn validate_agent_socket(path: &Path) -> Result<()> {
    validate_absolute(path, AGENT_SOCKET_LABEL)?;
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("inspect {AGENT_SOCKET_LABEL} {}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::{FileTypeExt, MetadataExt};
        ensure!(
            metadata.file_type().is_socket(),
            "{AGENT_SOCKET_LABEL} is not a socket"
        );
        ensure!(
            metadata.uid() == effective_uid(),
            "{AGENT_SOCKET_LABEL} has unsafe owner"
        );
    }
    Ok(())
}

pub(super) fn validate_socket_path(path: &Path) -> Result<()> {
    validate_absolute(path, "broker socket")?;
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        ensure!(
            path.as_os_str().as_bytes().len() < UNIX_SOCKET_PATH_MAX_BYTES,
            "broker socket path is too long: {}",
            path.display()
        );
    }
    Ok(())
}

pub(super) fn validate_absolute(path: &Path, label: &str) -> Result<()> {
    let valid = path.is_absolute()
        && path != Path::new(std::path::MAIN_SEPARATOR_STR)
        && path
            .components()
            .all(|part| !matches!(part, Component::CurDir | Component::ParentDir));
    ensure!(
        valid,
        "{label} must be a scoped absolute path: {}",
        path.display()
    );
    Ok(())
}

fn read_validated(
    path: &Path,
    label: &str,
    denied_mode: u32,
    allow_root: bool,
    limit: usize,
) -> Result<Vec<u8>> {
    read_validated_with(path, label, denied_mode, allow_root, limit, || Ok(()))
}

fn read_validated_with<F>(
    path: &Path,
    label: &str,
    denied_mode: u32,
    allow_root: bool,
    limit: usize,
    after_read: F,
) -> Result<Vec<u8>>
where
    F: FnOnce() -> Result<()>,
{
    validate_absolute(path, label)?;
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options
        .open(path)
        .with_context(|| format!("open {label} {}", path.display()))?;
    let before = inspect_secure_file(&file, path, label, denied_mode, allow_root, limit)?;
    let mut bytes = Vec::with_capacity(before.len() as usize);
    (&file)
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .with_context(|| format!("read {label} {}", path.display()))?;
    ensure!(bytes.len() <= limit, "{label} grew beyond its size limit");
    after_read()?;
    let after = inspect_secure_file(&file, path, label, denied_mode, allow_root, limit)?;
    verify_secure_file_stable(path, label, &before, &after, bytes.len())?;
    Ok(bytes)
}

#[cfg(test)]
pub(super) fn read_private_file_with_test_hook<F>(
    path: &Path,
    label: &str,
    after_read: F,
) -> Result<Vec<u8>>
where
    F: FnOnce() -> Result<()>,
{
    read_validated_with(
        path,
        label,
        PRIVATE_FILE_MODE_MASK,
        false,
        MAX_CONFIG_FILE_BYTES,
        after_read,
    )
}

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/runtime_artifact_bounds.rs"]
mod runtime_artifact_bounds;
