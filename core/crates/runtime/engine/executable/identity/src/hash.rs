use crate::constants::*;
use anyhow::{Context, Result, ensure};
use std::io::Read;
use yolo_build_image::MAX_EXECUTABLE_IMAGE_BYTES;
use yolo_crypto::Sha256Hasher;

pub(crate) fn read(reader: &mut impl Read, length: u64) -> Result<String> {
    ensure!(
        length > u64::default() && length <= MAX_EXECUTABLE_IMAGE_BYTES,
        INVALID_IMAGE
    );
    let mut buffer = [u8::default(); HASH_BUFFER_BYTES];
    let mut remaining = length;
    let mut digest = Sha256Hasher::new();
    while remaining > u64::default() {
        let wanted = remaining.min(HASH_BUFFER_BYTES as u64) as usize;
        let read = reader.read(&mut buffer[..wanted]).context(HASH_FAILED)?;
        ensure!(read != usize::default(), IMAGE_CHANGED);
        digest.update(&buffer[..read]);
        remaining -= read as u64;
    }
    let mut extra = [u8::default(); LENGTH_SENTINEL_BYTES];
    ensure!(
        reader.read(&mut extra).context(HASH_FAILED)? == usize::default(),
        IMAGE_CHANGED
    );
    Ok(digest.finish_hex())
}
