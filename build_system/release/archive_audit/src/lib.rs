use anyhow::{Context, Result, ensure};
use std::fs::{File, OpenOptions};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use yolo_release_foundation::file_seal::FileSeal;

mod central;
pub mod constants;
mod contract;
mod entry_path;
mod entry_type;
mod eocd;
mod local;
pub mod model;
mod payload;
mod source;
mod zlib;

use crate::source::ZipSource;
pub use model::{ArchiveEntry, ZipInventory};

const OPEN_CONTEXT: &str = "open ZIP archive without following a symlink";
const CHANGED_ERROR: &str = "ZIP archive changed during binary audit";

pub fn audit_archive_path(path: &Path) -> Result<ZipInventory> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .context(OPEN_CONTEXT)?;
    ensure!(
        file.metadata()?.is_file(),
        "ZIP archive must be a regular file"
    );
    audit_opened_archive(&file)
}

pub fn audit_opened_archive(file: &File) -> Result<ZipInventory> {
    let seal = FileSeal::capture(file)?;
    let inventory = audit_source(file)?;
    seal.verify(file, CHANGED_ERROR)?;
    Ok(inventory)
}

pub fn audit_bytes(bytes: &[u8]) -> Result<ZipInventory> {
    audit_source(&source::ByteSource(bytes))
}

fn audit_source(source: &dyn ZipSource) -> Result<ZipInventory> {
    let end = eocd::parse(source)?;
    let entries = central::parse(source, &end)?;
    contract::validate(&entries)?;
    let digests = local::validate(source, &entries, end.central_offset)?;
    Ok(ZipInventory {
        entries: entries
            .into_iter()
            .zip(digests)
            .map(|(entry, sha256)| ArchiveEntry {
                path: entry.path,
                mode: entry.mode,
                is_directory: entry.is_directory,
                crc32: entry.crc32,
                sha256,
                compressed_size: entry.compressed_size,
                uncompressed_size: entry.uncompressed_size,
            })
            .collect(),
    })
}
