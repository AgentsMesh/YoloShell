use crate::constants::*;
use anyhow::{Context, Result, ensure};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};
use yolo_build_image::{BuildImage, MAX_EXECUTABLE_IMAGE_BYTES};

const HEX_CHARS_PER_BYTE: usize = 2;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ServiceUnit {
    pub(crate) path: PathBuf,
    pub(crate) executable: PathBuf,
    pub(crate) image: BuildImage,
}

impl ServiceUnit {
    pub(crate) fn read(path: &Path) -> Result<Self> {
        let text = fs::read_to_string(path)
            .with_context(|| format!("read service unit {}", path.display()))?;
        let executable = field(&text, EXEC_START)?
            .split_whitespace()
            .next()
            .context(EMPTY_EXEC_START)?;
        let executable = PathBuf::from(executable);
        ensure!(executable.is_absolute(), ABSOLUTE_EXEC_START);
        ensure!(
            executable.file_name().and_then(|name| name.to_str()) == Some(IMAGE_NAME),
            WRONG_EXECUTABLE
        );
        ensure!(
            !fs::symlink_metadata(&executable)?.file_type().is_symlink(),
            MUTABLE_EXECUTABLE
        );
        let image = image(&executable)?;
        let declared_digest = field(&text, SERVICE_UNIT_IMAGE_DIGEST)?.trim();
        let declared_size = field(&text, SERVICE_UNIT_IMAGE_SIZE)?
            .trim()
            .parse::<u64>()?;
        ensure!(declared_digest == image.sha256(), DIGEST_MISMATCH);
        ensure!(declared_size == image.size_bytes(), SIZE_MISMATCH);
        Ok(Self {
            path: path.to_owned(),
            executable,
            image,
        })
    }
}

fn field<'a>(text: &'a str, prefix: &str) -> Result<&'a str> {
    text.lines()
        .find_map(|line| line.strip_prefix(prefix))
        .with_context(|| format!("service unit is missing {prefix}"))
}

pub(crate) fn image(path: &Path) -> Result<BuildImage> {
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
    )?;
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
