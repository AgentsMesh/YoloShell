use crate::{BuildImage, constants::IMAGE_CHANGED, fingerprint::Fingerprint, hash};
use anyhow::{Result, ensure};
use std::{fs::File, io::Seek};

pub(crate) fn capture(
    file: &mut File,
    mut verify: impl FnMut(&File) -> Result<()>,
) -> Result<(Fingerprint, BuildImage)> {
    let before = Fingerprint::inspect(&file.metadata()?)?;
    verify(file)?;
    file.rewind()?;
    let digest = hash::read(file, before.length)?;
    verify(file)?;
    ensure!(
        Fingerprint::inspect(&file.metadata()?)? == before,
        IMAGE_CHANGED
    );
    let image = BuildImage::new(digest, before.length)?;
    Ok((before, image))
}
