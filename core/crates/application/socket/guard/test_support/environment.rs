use super::constants::*;
use anyhow::{Context, Result, ensure};
use std::ffi::OsString;
use std::fs::Metadata;
use std::os::unix::fs::PermissionsExt;
use std::path::{Component, Path, PathBuf};

pub(super) fn resolve_executable() -> Result<PathBuf> {
    let source = required_environment(TEST_SOURCE_DIRECTORY_ENVIRONMENT)?;
    let workspace = required_environment(TEST_WORKSPACE_ENVIRONMENT)?;
    let relative = PathBuf::from(required_environment(TEST_EXECUTABLE_ENVIRONMENT)?);
    validate_relative(&relative)?;
    Ok(PathBuf::from(source).join(workspace).join(relative))
}

pub(super) fn canonical_executable(path: &Path) -> Result<PathBuf> {
    validate_executable(path)?;
    let canonical = path.canonicalize().context(EXECUTABLE_ERROR)?;
    validate_executable(&canonical)?;
    Ok(canonical)
}

pub(super) fn required_environment(name: &str) -> Result<OsString> {
    std::env::var_os(name).ok_or_else(|| anyhow::anyhow!("required environment is missing: {name}"))
}

pub(super) fn validate_relative(path: &Path) -> Result<()> {
    ensure!(
        !path.as_os_str().is_empty()
            && path
                .components()
                .all(|component| matches!(component, Component::Normal(_))),
        EXECUTABLE_ERROR
    );
    Ok(())
}

pub(super) fn validate_executable(path: &Path) -> Result<()> {
    let metadata = path.metadata().context(EXECUTABLE_ERROR)?;
    ensure!(
        metadata.is_file() && is_executable(&metadata),
        EXECUTABLE_ERROR
    );
    Ok(())
}

fn is_executable(metadata: &Metadata) -> bool {
    metadata.permissions().mode() & EXECUTABLE_MODE_MASK != u32::default()
}
