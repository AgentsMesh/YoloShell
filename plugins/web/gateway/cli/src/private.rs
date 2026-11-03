use anyhow::{Result, ensure};
use std::{io::Read, path::Path};
use yolo_paths::private_file::PrivateFileEntry;
use zeroize::Zeroizing;

const LIMIT_SENTINEL: u64 = 1;

pub(crate) fn read(path: &Path, limit: u64) -> Result<Zeroizing<Vec<u8>>> {
    yolo_paths::validation::validate_absolute(path)?;
    let entry = PrivateFileEntry::open(path)
        .map_err(|_| anyhow::anyhow!("web configuration requires private owned files"))?;
    let mut opened = entry.open_read(limit)?;
    let identity = opened.identity();
    let mut bytes = Zeroizing::new(Vec::new());
    opened
        .file_mut()
        .take(limit + LIMIT_SENTINEL)
        .read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() as u64 <= limit,
        "web configuration file exceeds its limit"
    );
    entry.verify_current_unchanged(identity, limit)?;
    Ok(bytes)
}
