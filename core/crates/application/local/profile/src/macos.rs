use anyhow::{Context, Result, ensure};
use std::{ffi::CStr, path::PathBuf};

pub(super) fn application_support_directory() -> Result<PathBuf> {
    decode(&super::foundation::application_support()?)
}

pub(crate) fn decode(representation: &CStr) -> Result<PathBuf> {
    let text = representation
        .to_str()
        .context(super::constants::FOUNDATION_UNAVAILABLE)?;
    ensure!(
        !text.is_empty() && !text.chars().any(char::is_control),
        super::constants::FOUNDATION_UNAVAILABLE
    );
    let path = PathBuf::from(text);
    yolo_paths::validation::validate_absolute(&path)?;
    Ok(path)
}
