use crate::constants::{USER_WRITE_PERMISSION, WRITE_PERMISSION_MASK};
use anyhow::{Result, ensure};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

pub fn set_read_only(path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    ensure!(
        !metadata.file_type().is_symlink(),
        "release snapshot contains a symlink"
    );
    if metadata.is_dir() {
        for entry in fs::read_dir(path)? {
            set_read_only(&entry?.path())?;
        }
    }
    let mode = metadata.permissions().mode() & !WRITE_PERMISSION_MASK;
    fs::set_permissions(path, fs::Permissions::from_mode(mode))?;
    Ok(())
}

pub fn restore_owner_write(path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() {
        return Ok(());
    }
    fs::set_permissions(
        path,
        fs::Permissions::from_mode(metadata.permissions().mode() | USER_WRITE_PERMISSION),
    )?;
    if metadata.is_dir() {
        for entry in fs::read_dir(path)? {
            restore_owner_write(&entry?.path())?;
        }
    }
    Ok(())
}
