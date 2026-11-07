use super::constants::errors;
use super::intake_constants::MAX_BYTES;
use crate::paths::identity::FileIdentity;
use crate::paths::private_file::PrivateFileEntry;
use anyhow::{Context, Result, ensure};
use serde::Serialize;
use std::io::Read;
use std::path::Path;

#[cfg(any(test, feature = "runtime-component-tests"))]
type RemoveHook = Box<dyn FnOnce() + Send>;

#[cfg(any(test, feature = "runtime-component-tests"))]
fn remove_hooks()
-> &'static std::sync::Mutex<std::collections::HashMap<std::path::PathBuf, RemoveHook>> {
    static HOOKS: std::sync::OnceLock<
        std::sync::Mutex<std::collections::HashMap<std::path::PathBuf, RemoveHook>>,
    > = std::sync::OnceLock::new();
    HOOKS.get_or_init(Default::default)
}

#[cfg(any(test, feature = "runtime-component-tests"))]
pub(crate) fn set_before_remove_hook(
    path: std::path::PathBuf,
    hook: impl FnOnce() + Send + 'static,
) {
    let previous = remove_hooks()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .insert(path, Box::new(hook));
    assert!(previous.is_none());
}

pub(super) fn create<T: Serialize>(
    directory: &crate::paths::directory::PrivateDirectory,
    path: &Path,
    value: &T,
) -> Result<FileIdentity> {
    create_with_fence(directory, path, value, || Ok(()))
}

pub(super) fn create_with_fence<T: Serialize>(
    directory: &crate::paths::directory::PrivateDirectory,
    path: &Path,
    value: &T,
    commit_fence: impl FnMut() -> Result<()>,
) -> Result<FileIdentity> {
    let encoded = serde_json::to_vec(value).context(errors::INTAKE_STORAGE)?;
    let entry = PrivateFileEntry::from_directory(
        directory.try_clone().context(errors::INTAKE_STORAGE)?,
        path,
    )?;
    entry
        .create_bytes_exclusive_with_fence(&encoded, MAX_BYTES, commit_fence)
        .context(errors::INTAKE_STORAGE)
}

pub(super) fn read_bytes(
    directory: &crate::paths::directory::PrivateDirectory,
    path: &Path,
) -> Result<(Vec<u8>, FileIdentity)> {
    let entry = PrivateFileEntry::from_directory(
        directory.try_clone().context(errors::INTAKE_STORAGE)?,
        path,
    )?;
    let mut opened = entry.open_read(MAX_BYTES).context(errors::INTAKE_STORAGE)?;
    let mut bytes = Vec::new();
    opened
        .file_mut()
        .by_ref()
        .take(MAX_BYTES.saturating_add(1))
        .read_to_end(&mut bytes)
        .context(errors::INTAKE_STORAGE)?;
    ensure!(
        u64::try_from(bytes.len())? <= MAX_BYTES,
        errors::INTAKE_STORAGE
    );
    opened.identity().verify_file_bounded(
        opened.file(),
        u64::try_from(bytes.len())?,
        path,
        MAX_BYTES,
    )?;
    entry.verify_current_unchanged(opened.identity(), MAX_BYTES)?;
    Ok((bytes, opened.identity()))
}

pub(super) fn remove_exact(
    directory: &crate::paths::directory::PrivateDirectory,
    path: &Path,
    identity: FileIdentity,
) -> Result<()> {
    remove_exact_with_fence(directory, path, identity, || Ok(()))
}

pub(super) fn remove_exact_with_fence<F>(
    directory: &crate::paths::directory::PrivateDirectory,
    path: &Path,
    identity: FileIdentity,
    mut fence: F,
) -> Result<()>
where
    F: FnMut() -> Result<()>,
{
    #[cfg(any(test, feature = "runtime-component-tests"))]
    run_before_remove_hook(path);
    fence()?;
    let removed = PrivateFileEntry::from_directory(
        directory.try_clone().context(errors::INTAKE_STORAGE)?,
        path,
    )?
    .remove_if_exact_with_fence(identity, MAX_BYTES, || fence())
    .context(errors::INTAKE_STORAGE)?;
    ensure!(removed, errors::INTAKE_STORAGE);
    fence()?;
    Ok(())
}

#[cfg(any(test, feature = "runtime-component-tests"))]
fn run_before_remove_hook(path: &Path) {
    let hook = remove_hooks()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .remove(path);
    if let Some(hook) = hook {
        hook();
    }
}
