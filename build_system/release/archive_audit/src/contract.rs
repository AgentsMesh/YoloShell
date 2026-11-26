use anyhow::{Result, ensure};

use crate::constants::ROOT_DIRECTORY_ENTRY;
use crate::model::CentralEntry;

pub(crate) fn validate(entries: &[CentralEntry]) -> Result<()> {
    ensure!(
        entries
            .iter()
            .any(|entry| entry.path == ROOT_DIRECTORY_ENTRY && entry.is_directory),
        "ZIP archive lacks the explicit Yolo.app root directory"
    );
    Ok(())
}
