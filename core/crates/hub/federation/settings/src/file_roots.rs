use crate::constants::INVALID;
use anyhow::{Result, ensure};
use std::collections::BTreeSet;
use yolo_hub_file_protocol::{FileRootConfiguration, MAX_FILE_ROOTS};

pub(super) fn validate(roots: &[FileRootConfiguration]) -> Result<()> {
    ensure!(roots.len() <= MAX_FILE_ROOTS, INVALID);
    let mut ids = BTreeSet::new();
    for root in roots {
        ensure!(ids.insert(root.fields().root_id), INVALID);
    }
    Ok(())
}
