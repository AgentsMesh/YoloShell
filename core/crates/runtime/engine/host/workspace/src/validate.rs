use crate::command;
use crate::error::WorktreeError;
use std::path::{Path, PathBuf};

pub(crate) fn validate_id(value: &str) -> Result<(), WorktreeError> {
    if value.is_empty()
        || value.len() > 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return Err(WorktreeError::InvalidRepository(value.into()));
    }
    Ok(())
}

pub(crate) fn validate_branch(value: &str) -> Result<(), WorktreeError> {
    if value.is_empty()
        || value.len() > 128
        || value.starts_with('-')
        || value.starts_with('/')
        || value.ends_with('/')
        || value.contains("..")
        || value.contains('\0')
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b'/'))
    {
        return Err(WorktreeError::InvalidBranch);
    }
    Ok(())
}

pub(crate) fn repository_root(executable: &Path, root: &Path) -> Result<PathBuf, WorktreeError> {
    if !root.is_absolute() {
        return Err(WorktreeError::InvalidRepository(
            "repository root must be absolute".into(),
        ));
    }
    let canonical = std::fs::canonicalize(root)
        .map_err(|_| WorktreeError::InvalidRepository(root.display().to_string()))?;
    let output = command::run(
        executable,
        &crate::constants::GIT_TOPLEVEL_ARGUMENTS
            .iter()
            .map(|argument| (*argument).to_owned())
            .collect::<Vec<_>>(),
        &canonical,
    )
    .map_err(|_| WorktreeError::InvalidRepository(canonical.display().to_string()))?;
    let top = std::fs::canonicalize(output.trim())
        .map_err(|_| WorktreeError::InvalidRepository(canonical.display().to_string()))?;
    if top != canonical {
        return Err(WorktreeError::InvalidRepository(
            canonical.display().to_string(),
        ));
    }
    Ok(canonical)
}
