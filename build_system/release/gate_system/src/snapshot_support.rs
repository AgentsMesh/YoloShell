use anyhow::{Context, Result};
use std::path::Path;

pub(crate) fn utf8(path: &Path) -> Result<String> {
    Ok(path
        .to_str()
        .context("release gate path must be UTF-8")?
        .to_owned())
}
