use anyhow::{Result, bail};
use std::path::{Component, Path, PathBuf};

pub fn validate_home(home: &Path) -> Result<()> {
    if !home.is_absolute() || home == Path::new(std::path::MAIN_SEPARATOR_STR) {
        bail!("refusing unsafe Yolo home: {}", home.display());
    }
    validate_absolute(home)
}

pub fn validate_absolute(path: &Path) -> Result<()> {
    if !path.is_absolute() {
        bail!("path must be absolute: {}", path.display());
    }
    let mut normalized = PathBuf::from(std::path::MAIN_SEPARATOR_STR);
    for component in path.components().skip(1) {
        match component {
            Component::Normal(name) => normalized.push(name),
            _ => bail!("path is not lexically normalized: {}", path.display()),
        }
    }
    if normalized.as_os_str() != path.as_os_str() {
        bail!("path is not lexically normalized: {}", path.display());
    }
    Ok(())
}
