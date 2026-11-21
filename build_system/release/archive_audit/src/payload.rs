use anyhow::{Context, Result, bail, ensure};
use crc32fast::Hasher;
use sha2::{Digest, Sha256};

use crate::constants::{DEFLATE_METHOD, PAYLOAD_BUFFER_BYTES, STORED_METHOD};
use crate::model::CentralEntry;
use crate::source::{ZipSource, bytes};
use crate::zlib;
use yolo_release_foundation::digest::hex_digest;

pub(crate) fn validate(
    source: &dyn ZipSource,
    entry: &CentralEntry,
    data_offset: u64,
) -> Result<String> {
    match entry.method {
        STORED_METHOD => validate_stored(source, entry, data_offset),
        DEFLATE_METHOD => validate_deflate(source, entry, data_offset),
        _ => bail!("unreachable ZIP payload method"),
    }
}

fn validate_stored(
    source: &dyn ZipSource,
    entry: &CentralEntry,
    data_offset: u64,
) -> Result<String> {
    let mut crc = Hasher::new();
    let mut digest = Sha256::new();
    let mut offset = data_offset;
    let mut remaining = entry.compressed_size;
    let mut buffer = [0_u8; PAYLOAD_BUFFER_BYTES];
    while remaining > 0 {
        let length = usize::try_from(remaining.min(buffer.len() as u64))?;
        source.read_exact(offset, &mut buffer[..length])?;
        crc.update(&buffer[..length]);
        digest.update(&buffer[..length]);
        offset = offset
            .checked_add(length as u64)
            .context("stored ZIP payload offset overflow")?;
        remaining -= length as u64;
    }
    ensure!(crc.finalize() == entry.crc32, "ZIP payload CRC mismatch");
    Ok(hex_digest(digest.finalize()))
}

fn validate_deflate(
    source: &dyn ZipSource,
    entry: &CentralEntry,
    data_offset: u64,
) -> Result<String> {
    let length = usize::try_from(entry.compressed_size).context("deflate payload length")?;
    let mut compressed = bytes(source, data_offset, length)?;
    zlib::validate(&mut compressed, entry.uncompressed_size, entry.crc32)
}
