use crate::constants::*;
use anyhow::{Context, Result, ensure};
use std::{
    fs,
    path::{Path, PathBuf},
};
use yolo_build_image::BuildImage;

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
    yolo_local_lifecycle::image(path)
}
