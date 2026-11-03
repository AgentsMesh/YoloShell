use anyhow::{Context, Result, ensure};
use sha2::{Digest, Sha256};
use std::fmt::Write;
use std::fs::File;
use std::io::Read;
use std::os::unix::fs::{FileExt, MetadataExt};

pub const DIGEST_BUFFER_SIZE: usize = 64 * 1024;
pub const MAX_DIGEST_INPUT_BYTES: u64 = 1024 * 1024 * 1024;
const HEX_CHARACTERS_PER_BYTE: usize = 2;

pub fn opened_file_sha256(file: &File) -> Result<String> {
    opened_file_sha256_with(file, || Ok(()))
}

pub fn copy_opened_bounded(
    source: &File,
    destination: &mut File,
    maximum_bytes: u64,
    description: &str,
) -> Result<()> {
    let before = source.metadata()?;
    ensure!(before.is_file(), "{description} must be a regular file");
    ensure!(before.len() <= maximum_bytes, "{description} is too large");
    let copied = std::io::copy(&mut source.take(before.len()), destination)?;
    ensure!(
        copied == before.len(),
        "{description} was truncated while copied"
    );
    ensure!(
        stable(&before, &source.metadata()?),
        "{description} changed while copied"
    );
    Ok(())
}

fn opened_file_sha256_with(
    file: &File,
    after_capture: impl FnOnce() -> Result<()>,
) -> Result<String> {
    let before = file.metadata()?;
    ensure!(before.is_file(), "digest input must be a regular file");
    ensure!(
        before.len() <= MAX_DIGEST_INPUT_BYTES,
        "digest input exceeded the bounded limit"
    );
    after_capture()?;
    let mut digest = Sha256::new();
    let mut offset = 0_u64;
    let mut buffer = [0_u8; DIGEST_BUFFER_SIZE];
    while offset < before.len() {
        let remaining = usize::try_from((before.len() - offset).min(DIGEST_BUFFER_SIZE as u64))?;
        let count = file
            .read_at(&mut buffer[..remaining], offset)
            .context("read opened release artifact")?;
        ensure!(count > 0, "digest input was truncated while it was read");
        digest.update(&buffer[..count]);
        offset += count as u64;
    }
    ensure!(
        stable(&before, &file.metadata()?),
        "digest input changed while it was read"
    );
    Ok(hex_digest(digest.finalize()))
}

#[cfg(test)]
pub(crate) fn opened_file_sha256_after_capture(
    file: &File,
    after_capture: impl FnOnce() -> Result<()>,
) -> Result<String> {
    opened_file_sha256_with(file, after_capture)
}

fn stable(before: &std::fs::Metadata, after: &std::fs::Metadata) -> bool {
    before.dev() == after.dev()
        && before.ino() == after.ino()
        && before.len() == after.len()
        && before.mtime() == after.mtime()
        && before.mtime_nsec() == after.mtime_nsec()
        && before.ctime() == after.ctime()
        && before.ctime_nsec() == after.ctime_nsec()
}

pub fn bytes_sha256(bytes: &[u8]) -> String {
    hex_digest(Sha256::digest(bytes))
}

pub fn hex_digest(digest: impl IntoIterator<Item = u8>) -> String {
    let bytes = digest.into_iter();
    let mut value = String::with_capacity(bytes.size_hint().0 * HEX_CHARACTERS_PER_BYTE);
    for byte in bytes {
        write!(&mut value, "{byte:02x}").expect("writing to String cannot fail");
    }
    value
}
