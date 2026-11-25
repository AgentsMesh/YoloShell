use crate::constants::*;
use anyhow::{Result, ensure};
use std::path::Path;

pub fn validate_install_root(path: &Path) -> Result<()> {
    let text = path.to_str().ok_or_else(|| anyhow::anyhow!(INVALID_ROOT))?;
    ensure!(
        text.len() <= MAX_INSTALL_PATH_BYTES && !text.chars().any(char::is_control),
        INVALID_ROOT
    );
    let relative = text
        .strip_prefix('/')
        .ok_or_else(|| anyhow::anyhow!(INVALID_ROOT))?;
    ensure!(
        !relative.is_empty()
            && relative
                .split('/')
                .all(|part| !part.is_empty() && part != "." && part != ".."),
        INVALID_ROOT
    );
    Ok(())
}
