use anyhow::{Context, Result, ensure};
use sha2::{Digest, Sha256};
use std::{fs, io::Read, path::Path};
use yolo_build_image::{BuildImage, MAX_EXECUTABLE_IMAGE_BYTES};

const INVALID_EXECUTABLE: &str = "service executable is not a regular file";
const INVALID_EXECUTABLE_SIZE: &str = "service executable size is invalid";
const EXECUTABLE_CHANGED: &str = "service executable changed while hashing";
const HEX_CHARS_PER_BYTE: usize = 2;

pub fn image(path: &Path) -> Result<BuildImage> {
    let metadata = fs::symlink_metadata(path)?;
    ensure!(metadata.file_type().is_file(), INVALID_EXECUTABLE);
    ensure!(
        metadata.len() > 0 && metadata.len() <= MAX_EXECUTABLE_IMAGE_BYTES,
        INVALID_EXECUTABLE_SIZE
    );
    let mut file = fs::File::open(path)?;
    let mut digest = Sha256::new();
    let copied = std::io::copy(
        &mut file.by_ref().take(metadata.len() + 1),
        &mut DigestWriter(&mut digest),
    )
    .with_context(|| EXECUTABLE_CHANGED.to_owned())?;
    ensure!(copied == metadata.len(), EXECUTABLE_CHANGED);
    let after = file.metadata()?;
    ensure!(after.len() == metadata.len(), EXECUTABLE_CHANGED);
    BuildImage::new(hex(digest.finalize()), metadata.len()).map_err(Into::into)
}

fn hex(bytes: impl AsRef<[u8]>) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    bytes.as_ref().iter().fold(
        String::with_capacity(bytes.as_ref().len() * HEX_CHARS_PER_BYTE),
        |mut result, byte| {
            result.push(DIGITS[(byte >> 4) as usize] as char);
            result.push(DIGITS[(byte & 0x0f) as usize] as char);
            result
        },
    )
}

struct DigestWriter<'a>(&'a mut Sha256);
impl std::io::Write for DigestWriter<'_> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.update(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
